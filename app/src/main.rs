use std::f32::consts::PI;

use bevy::{
    asset::LoadState,
    audio::Volume,
    audio::{PlaybackMode, VolumeLevel},
    prelude::*,
};
use bevy_egui::EguiPlugin;
use bevy_ggrs::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_matchbox::{prelude::SingleChannel, MatchboxSocket};

mod animation;
mod camera;
mod collision;
mod component;
mod gui;
mod input;
mod map;
mod p2p;
mod rand;

use animation::*;
use collision::*;
use component::*;
use input::*;
use map::*;
use p2p::*;
use rand::Rng;

#[derive(States, Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GameState {
    #[default]
    Loading,
    Lobby,
    Connecting,
    Game,
}

#[derive(States, Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DebugState {
    On,
    #[default]
    Off,
}

#[derive(Resource)]
struct LoadingAssets(Vec<HandleUntyped>);

#[derive(Resource, Debug, Reflect, Default)]
struct GameFrameCount(u64);

pub const MAP_Z: f32 = 0.;
pub const PLAYER_Z: f32 = 10.;
pub const BULLET_Z: f32 = 15.;
pub const MAP_FG_Z: f32 = 20.;

fn main() {
    let mut app = App::new();

    app.insert_resource(Msaa::Off)
        .insert_resource(LoadingAssets(vec![]))
        .register_type::<WallContactState>()
        .register_type::<Velocity>()
        .register_type::<InputAngle>()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    // window config
                    primary_window: Some(Window {
                        prevent_default_event_handling: true,
                        fit_canvas_to_parent: true,
                        title: "p2pg".to_owned(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    asset_folder: "assets".to_owned(),
                    ..default()
                }),
        )
        .add_plugins(map::TiledPlugin)
        .add_plugins(SpriteAnimationPlugin)
        .add_plugins(DebugHitboxPlugin)
        .add_plugins(NetworkingPlugin)
        .add_plugins(TouchPlugin)
        .add_state::<GameState>()
        .add_state::<DebugState>()
        // LOADING
        .add_systems(OnEnter(GameState::Loading), load) // load essential assets
        .add_systems(Update, check_load.run_if(in_state(GameState::Loading))) // transition state when assets loaded
        .add_systems(
            OnExit(GameState::Loading),
            (camera::spawn_primary, camera::spawn_minimap_camera).chain(),
        ) // pre-connect initialization (camera, bg, etc.)
        // LOBBY
        .add_systems(OnEnter(GameState::Lobby), unload_game)
        .add_systems(Update, gui::main_menu.run_if(in_state(GameState::Lobby)))
        // CONNECTING
        .add_systems(OnEnter(GameState::Connecting), setup_socket)
        .add_systems(
            Update,
            (wait_for_players, gui::connecting).run_if(in_state(GameState::Connecting)),
        ) // "lobby" -> waits for other player(s) and then transitions to countdown
        .add_systems(
            OnEnter(GameState::Game),
            (spawn_players, reset_frame_count).chain(),
        ) // spawn players once connected
        // COMBAT
        .add_systems(
            GgrsSchedule,
            (
                restart_on_death,
                first_frame_init, // runs only if frame_count is 0
                sense_walls,
                move_player,
                collision::player_terrain_system,
                track_player_facing,
                point_bow,
                shoot,
                //collision::bullet_terrain_system,
                collision::bullet_player_system,
                award_points,
                reload,
                move_bullets,
                despawn_after_lifetime,
                increment_frame_count,
            )
                .chain()
                .run_if(in_state(GameState::Game)),
        ) // synchronized p2p combat system ("the gameplay")
        // MISC
        .add_systems(
            Update,
            (
                toggle_debug,
                camera::follow_player,
                animate_player,
                animate_bow,
                process_ggrs_events,
                gui::points_display.run_if(in_state(GameState::Game)),
                gui::fps_display.run_if(in_state(DebugState::On)),
            ),
        ) // client-side non-deterministic systems
        .add_plugins(EguiPlugin)
        .add_plugins(WorldInspectorPlugin::new().run_if(in_state(DebugState::On)))
        .run();
}

fn award_points(
    mut commands: Commands,
    q_damaged: Query<(Entity, &LastDamagedBy)>,
    mut q_player: Query<(&Player, &mut Points)>,
) {
    for (victim, damager) in &q_damaged {
        for (attacker, mut points) in &mut q_player {
            if attacker.id == damager.id {
                // this is the player that shot the damaging bullet
                points.0 += 100;
                commands.entity(victim).remove::<LastDamagedBy>();
            }
        }
    }
}

fn restart_on_death(mut fc: ResMut<GameFrameCount>, q_player: Query<&Health, With<Player>>) {
    for health in &q_player {
        if health.0 <= 0 {
            fc.0 = 0;
        }
    }
}

fn first_frame_init(
    mut commands: Commands,
    fc: Res<GameFrameCount>,
    mut q_player: Query<(Entity, &mut Transform), With<Player>>,
    q_bullet: Query<Entity, With<Bullet>>,
    q_spawns: Query<&GlobalTransform, (With<Spawnpoint>, Without<Player>)>,
    mut rng: ResMut<Rng>,
) {
    if fc.0 != 0 {
        return;
    }

    // fetch all map spawnpoints
    let mut spawns: Vec<Vec2> = q_spawns
        .iter()
        .map(|gt| gt.translation().truncate())
        .collect();

    let player_iter = q_player.iter_mut();
    assert!(spawns.len() >= player_iter.len());

    // for every player...
    for (player, mut transform) in player_iter {
        // reset core components
        commands.entity(player).insert(BasePlayerBundle::default());

        //.. move to a random spawn point
        let spawn = rng.extract_random(&mut spawns);
        transform.translation.x = spawn.x;
        transform.translation.y = spawn.y;
    }

    // despawn all bullets
    for bullet in &q_bullet {
        commands.entity(bullet).despawn_recursive();
    }
}

fn increment_frame_count(mut fc: ResMut<GameFrameCount>) {
    fc.0 += 1;
}

fn toggle_debug(
    state: Res<State<DebugState>>,
    mut next_state: ResMut<NextState<DebugState>>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Slash) {
        next_state.set(match state.get() {
            DebugState::Off => DebugState::On,
            DebugState::On => DebugState::Off,
        });
    }
}

fn unload_game(
    mut commands: Commands,
    q_player: Query<Entity, With<Player>>,
    q_bullet: Query<Entity, With<Bullet>>,
) {
    // remove all players and bullets
    for e in &q_player {
        commands.entity(e).despawn_recursive();
    }
    for e in &q_bullet {
        commands.entity(e).despawn_recursive();
    }
    // reset rng
    commands.insert_resource(Rng::new(8008135));
    // remove any sockets and sessions
    commands.remove_resource::<MatchboxSocket<SingleChannel>>();
    commands.remove_resource::<Session<GgrsConfig>>();
}

fn load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading: ResMut<LoadingAssets>,
) {
    [
        "Archer.png",
        "arrow.png",
        "bow.png",
        "sfx/Bow_Release.wav",
        "sfx/Damage_1.wav",
        "snowy.tmx",
        "tilesets/Set_A_Darkwoods1.png",
    ]
    .into_iter()
    .for_each(|asset| {
        loading.0.push(asset_server.load_untyped(asset));
    });

    // load the tilemap
    commands
        .spawn(TilemapLoaderBundle::new("snowy.tmx"))
        .insert(Transform::from_translation(
            (-16. * 25., -16. * 25., MAP_Z).into(),
        ));
}

fn check_load(
    loading: Res<LoadingAssets>,
    asset_server: Res<AssetServer>,
    mut _commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    q_tilemap_loaders: Query<&TilemapLoader>,
) {
    let assets_loaded: bool =
        match asset_server.get_group_load_state(loading.0.iter().map(|h| h.id())) {
            LoadState::Failed => {
                panic!("Could not load assets...");
            }
            LoadState::Loaded => true,
            _ => {
                info!("Loading assets...");
                false
            }
        };
    let tilemaps_loaded: bool = q_tilemap_loaders.is_empty();
    if assets_loaded && tilemaps_loaded {
        next_state.set(GameState::Lobby);
    }
}

fn move_bullets(mut q_bullets: Query<(&Velocity, &mut Transform), With<Bullet>>) {
    for (vel, mut bullet_transform) in &mut q_bullets {
        bullet_transform.translation.x += vel.0.x;
        bullet_transform.translation.y += vel.0.y;
    }
}

fn shoot(
    mut commands: Commands,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut q_player: Query<(&Player, &Transform, &mut CanShoot)>,
    asset_server: Res<AssetServer>,
) {
    const SHOOT_COOLDOWN: usize = 25;

    let bullet_handle = asset_server.load("arrow.png");

    for (player, player_transform, mut can_shoot) in &mut q_player {
        let (input, _) = inputs[player.id];

        if input.fire() && can_shoot.value && can_shoot.since_last >= SHOOT_COOLDOWN {
            can_shoot.value = false;
            can_shoot.since_last = 0;

            let angle = from_u8_angle(input.angle);
            let dir = angle_to_vec(angle);
            let mut arrow_angle = 2. * PI - angle;
            arrow_angle += PI / 4.;

            let pos = player_transform.translation.truncate();

            commands
                .spawn(BulletBundle::new(
                    player.id,
                    dir,
                    2.5,
                    150,
                    bullet_handle.clone(),
                ))
                .insert(
                    Transform::from_xyz(pos.x + dir.x * 16., pos.y + dir.y * 16., BULLET_Z)
                        .with_rotation(Quat::from_rotation_z(arrow_angle)),
                )
                .add_rollback();

            commands.spawn(AudioBundle {
                source: asset_server.load("sfx/Bow_Release.wav"),
                settings: PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Relative(VolumeLevel::new(0.2)),
                    speed: 2.,
                    ..default()
                },
            });
        }
    }
}

fn reload(
    mut _commands: Commands,
    _asset_server: Res<AssetServer>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut q_player: Query<(&Player, &mut CanShoot)>,
) {
    for (player, mut can_shoot) in &mut q_player {
        let (input, _) = inputs[player.id];

        can_shoot.since_last += 1;

        if !input.fire() {
            if !can_shoot.value {
                can_shoot.value = true;
            }
        }
    }
}

fn point_bow(
    q_player: Query<&Player>,
    mut q_bow: Query<(&mut Transform, &Parent), With<Bow>>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for (mut transform, parent) in &mut q_bow {
        let Ok(player) = q_player.get(parent.get()) else {
            continue;
        };
        let (input, _) = inputs[player.id];

        let mut angle = 2. * PI - from_u8_angle(input.angle);
        angle += PI / 4.;

        transform.rotation = Quat::from_rotation_z(angle);
    }
}

fn track_player_facing(
    mut q_player: Query<(&Player, &mut Facing)>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for (player, mut facing) in &mut q_player {
        let (input, _) = inputs[player.id];

        if input.angle < 32 {
            *facing = Facing::Up;
        } else if input.angle < 96 {
            *facing = Facing::Right;
        } else if input.angle < 160 {
            *facing = Facing::Down;
        } else if input.angle < 224 {
            *facing = Facing::Left;
        } else {
            *facing = Facing::Up;
        }
    }
}

fn animate_bow(
    q_player: Query<&CanShoot, (With<Player>, Changed<CanShoot>)>,
    mut q_bow: Query<(&mut AnimationIndices, &Parent), With<Bow>>,
) {
    for (mut bow_indices, parent) in &mut q_bow {
        let Ok(can_shoot) = q_player.get(parent.get()) else {
            continue;
        };
        let new_indices = if can_shoot.since_last > 10 {
            BowAnimation::Draw
        } else {
            BowAnimation::Empty
        }
        .into();
        if *bow_indices != new_indices {
            *bow_indices = new_indices;
        }
    }
}

fn animate_player(mut q_player: Query<(&Velocity, &Facing, &mut AnimationIndices), With<Player>>) {
    for (velocity, facing, mut indices) in &mut q_player {
        let new_indices = player_animation_indices(velocity.0, facing);
        if *indices != new_indices {
            *indices = new_indices;
        }
    }
}

fn sense_walls(
    mut q_player: Query<
        (&WallSensors, &mut WallContactState, &Transform),
        (With<Player>, Without<RigidBody>),
    >,
    q_rigidbody: Query<(&Hitbox, &GlobalTransform), (With<RigidBody>, Without<Player>)>,
) {
    for (p_wallsensors, mut walls, p_transform) in &mut q_player {
        let mut hitting_up = false;
        let mut hitting_down = false;
        let mut hitting_right = false;
        let mut hitting_left = false;
        for (r_hitbox, r_transform) in &q_rigidbody {
            let rt = r_transform.compute_transform();
            if !hitting_up {
                hitting_up = hitbox_intersects((&p_wallsensors.up, p_transform), (r_hitbox, &rt));
            }
            if !hitting_down {
                hitting_down =
                    hitbox_intersects((&p_wallsensors.down, p_transform), (r_hitbox, &rt));
            }
            if !hitting_left {
                hitting_left =
                    hitbox_intersects((&p_wallsensors.left, p_transform), (r_hitbox, &rt));
            }
            if !hitting_right {
                hitting_right =
                    hitbox_intersects((&p_wallsensors.right, p_transform), (r_hitbox, &rt));
            }
        }
        walls.up = hitting_up;
        walls.down = hitting_down;
        walls.left = hitting_left;
        walls.right = hitting_right;
    }
}

fn wall_direction_clamp(mut dir: Vec2, walls: &WallContactState) -> Vec2 {
    if (walls.up && dir.y > 0.) || (walls.down && dir.y < 0.) {
        dir.y = 0.;
    }
    if (walls.right && dir.x > 0.) || (walls.left && dir.x < 0.) {
        dir.x = 0.;
    }
    dir
}

fn move_player(
    mut q_player: Query<(&mut Transform, &mut Velocity, &WallContactState, &Player)>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    // the epsilon around which input axis are snapped to 0
    const SNAP_TO_AXIS: f32 = 0.02;
    for (mut transform, mut velocity, walls, player) in &mut q_player {
        let (input, _) = inputs[player.id];

        velocity.0 = if !input.moving() {
            Vec2::ZERO
        } else {
            let mut input_dir = input.direction();
            if input_dir.x.abs() <= SNAP_TO_AXIS {
                input_dir.x = 0.
            }
            if input_dir.y.abs() <= SNAP_TO_AXIS {
                input_dir.y = 0.
            }
            wall_direction_clamp(input_dir, walls).normalize_or_zero() * 1.4
        };

        transform.translation += velocity.0.extend(0.);
    }
}

fn despawn_after_lifetime(mut commands: Commands, mut query: Query<(Entity, &mut Lifetime)>) {
    for (entity, mut lifetime) in &mut query {
        lifetime.0 -= 1;
        if lifetime.0 == 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn reset_frame_count(mut commands: Commands) {
    commands.insert_resource(GameFrameCount(0));
}

fn spawn_players(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
) {
    let player_image = asset_server.load("Archer.png");
    let player_atlas =
        TextureAtlas::from_grid(player_image.clone(), Vec2::splat(16.), 24, 1, None, None);
    let player_atlas_handle = atlases.add(player_atlas);

    let bow_image: Handle<Image> = asset_server.load("bow.png");
    let bow_atlas = TextureAtlas::from_grid(bow_image.clone(), Vec2::splat(16.), 2, 2, None, None);
    let bow_atlas_handle = atlases.add(bow_atlas);

    commands
        .spawn(PlayerBundle::new(0, player_atlas_handle.clone()))
        .insert(Transform::from_xyz(-16., 0., PLAYER_Z))
        .with_children(|parent| {
            parent
                .spawn(BowBundle::new(bow_atlas_handle.clone()))
                .add_rollback();
        })
        .add_rollback();
    commands
        .spawn(PlayerBundle::new(1, player_atlas_handle.clone()))
        .insert(Transform::from_xyz(16., 0., PLAYER_Z))
        .with_children(|parent| {
            parent
                .spawn(BowBundle::new(bow_atlas_handle.clone()))
                .add_rollback();
        })
        .add_rollback();
}
