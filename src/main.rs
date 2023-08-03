use std::f32::consts::PI;

use bevy::{
    asset::LoadState,
    audio::Volume,
    audio::{PlaybackMode, VolumeLevel},
    prelude::*,
    render::{camera::ScalingMode, primitives::Aabb},
};
use bevy_ggrs::*;

mod animation;
mod collision;
mod component;
mod input;
mod map;
mod p2p;
mod rand;

use animation::*;
use collision::*;
use component::*;
use input::{angle_to_vec, from_u8_angle};
use map::*;
use p2p::*;
use rand::Rng;

#[derive(States, Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GameState {
    #[default]
    Loading,
    Connecting,
    Countdown,
    Combat,
}

#[derive(Resource)]
struct LoadingAssets(Vec<HandleUntyped>);

const MAP_Z: f32 = 0.;
const PLAYER_Z: f32 = 10.;
const BULLET_Z: f32 = 15.;

fn main() {
    let mut app = App::new();

    GgrsPlugin::<GgrsConfig>::new()
        .with_input_system(input::input)
        .with_update_frequency(60)
        .register_rollback_component::<Transform>()
        .register_rollback_component::<CanShoot>()
        .register_rollback_component::<Velocity>()
        .register_rollback_component::<Lifetime>()
        .register_rollback_component::<InputAngle>()
        .register_rollback_resource::<Rng>()
        .build(&mut app);

    app.insert_resource(Msaa::Off)
        .insert_resource(LoadingAssets(vec![]))
        .insert_resource(Rng::new(8008135)) // heh
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    // window config
                    primary_window: Some(Window {
                        prevent_default_event_handling: false,
                        fit_canvas_to_parent: true,
                        title: "p2pg".to_owned(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(map::TiledPlugin)
        .add_plugins(SpriteAnimationPlugin)
        //.add_plugins(DebugHitboxPlugin)
        .add_state::<GameState>()
        .add_systems(OnEnter(GameState::Loading), load) // load essential assets
        .add_systems(Update, check_load.run_if(in_state(GameState::Loading))) // transition state when assets loaded
        .add_systems(OnExit(GameState::Loading), (setup, setup_socket)) // pre-connect initialization (camera, bg, etc.)
        .add_systems(
            Update,
            wait_for_players.run_if(in_state(GameState::Connecting)),
        ) // "lobby" -> waits for other player(s) and then transitions to countdown
        .add_systems(OnExit(GameState::Connecting), spawn_players) // spawn players once connected
        .add_systems(
            GgrsSchedule,
            countdown.run_if(in_state(GameState::Countdown)),
        ) //TODO: pre-game countdown system
        .add_systems(
            GgrsSchedule,
            (
                track_player_facing,
                point_bow,
                shoot,
                move_player,
                bullet_player_collisions,
                reload,
                move_bullets,
                despawn_after_lifetime,
            )
                .chain()
                .run_if(in_state(GameState::Combat)),
        ) // synchronized p2p combat system ("the gameplay")
        .add_systems(Update, (camera_follow, animate_player, animate_bow)) // client-side non-deterministic systems
        .add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin)
        //.add_plugins(WorldInspectorPlugin::new())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    maps: Res<Assets<TiledMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let ideal_aspect_ratio = 16f32 / 9.;
    let max_width = 16. * 25.;
    let max_height = max_width / ideal_aspect_ratio;

    // main camera
    commands
        .spawn(MainCamera)
        .insert(Camera2dBundle {
            projection: OrthographicProjection {
                // optimize view area on 16
                scaling_mode: ScalingMode::AutoMax {
                    max_height,
                    max_width,
                },
                ..default()
            },
            camera: Camera {
                order: 0,
                ..default()
            },
            ..default()
        })
        .insert(FollowPlayer);

    // minimap camera ( BORKED ! )
    // https://github.com/bevyengine/bevy/issues/9340
    /*commands.spawn(MinimapCamera).insert(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::Fixed { width: 100., height: 100. },
            ..default()
        },
        camera: Camera {
            viewport: Some(Viewport {
                physical_size: UVec2::splat(100),
                ..default()
            }),
            order: 1,
            ..default()
        },
        ..default()
    }).insert(FollowPlayer);*/

    // spawn the background tilemap
    let tilemap = &maps.get(&asset_server.load("basic.tmx")).unwrap().0;
    let mesh = map::tilemap_to_mesh(tilemap);
    let mesh_handle = meshes.add(mesh);
    let tileset_handle = asset_server.load("OverworldTileset_v03.png");
    let material = ColorMaterial {
        texture: Some(tileset_handle),
        color: Color::WHITE,
    };
    let material_handle = materials.add(material);

    commands
        .spawn(TilemapBundle::new(mesh_handle, material_handle))
        .insert(
            Transform::from_scale(Vec3::splat(0.5))
                .with_translation((-16. * 25., -16. * 25., MAP_Z).into()),
        );
}

fn load(asset_server: Res<AssetServer>, mut loading: ResMut<LoadingAssets>) {
    let tileset: Handle<Image> = asset_server.load("OverworldTileset_v03.png");
    let tilemap: Handle<TiledMap> = asset_server.load("basic.tmx");
    let player: Handle<Image> = asset_server.load("Archer.png");
    let arrow: Handle<Image> = asset_server.load("arrow.png");
    let bow: Handle<Image> = asset_server.load("bow.png");
    let bow_charge: Handle<AudioSource> = asset_server.load("sfx/Bow_Charge.wav");
    let bow_release: Handle<AudioSource> = asset_server.load("sfx/Bow_Release.wav");

    loading.0.push(tileset.clone_untyped());
    loading.0.push(tilemap.clone_untyped());
    loading.0.push(player.clone_untyped());
    loading.0.push(arrow.clone_untyped());
    loading.0.push(bow.clone_untyped());
    loading.0.push(bow_charge.clone_untyped());
    loading.0.push(bow_release.clone_untyped());
}

fn check_load(
    loading: Res<LoadingAssets>,
    asset_server: Res<AssetServer>,
    mut _commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    match asset_server.get_group_load_state(loading.0.iter().map(|h| h.id())) {
        LoadState::Failed => {
            panic!("Could not load assets...");
        }
        LoadState::Loaded => {
            next_state.set(GameState::Connecting);
        }
        _ => {
            info!("Loading assets...");
        }
    }
}

fn countdown(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Combat);
}

fn bullet_player_collisions(
    mut commands: Commands,
    q_bullet: Query<(Entity, &Hitbox, &Transform), (With<Bullet>, Without<Player>)>,
    q_player: Query<(Entity, &Hitbox, &Transform), (With<Player>, Without<Bullet>)>,
) {
    for (_p_entity, p_hitbox, p_transform) in &q_player {
        for (b_entity, b_hitbox, b_transform) in &q_bullet {
            if hitbox_intersects(
                (&p_hitbox.shape, p_transform),
                (&b_hitbox.shape, b_transform),
            ) {
                commands.entity(b_entity).despawn();
            }
        }
    }
}

fn move_bullets(mut q_bullets: Query<(&Bullet, &mut Transform)>) {
    for (bullet, mut bullet_transform) in &mut q_bullets {
        let delta = bullet.dir * bullet.vel;
        bullet_transform.translation.x += delta.x;
        bullet_transform.translation.y += delta.y;
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
                .spawn(BulletBundle::new(dir, 2.2, 150, bullet_handle.clone()))
                .insert(
                    Transform::from_xyz(pos.x + dir.x * 16., pos.y + dir.y * 16., BULLET_Z)
                        .with_rotation(Quat::from_rotation_z(arrow_angle)),
                )
                .add_rollback();

            commands.spawn(AudioBundle {
                source: asset_server.load("sfx/Bow_Release.wav"),
                settings: PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Relative(VolumeLevel::new(0.1)),
                    ..default()
                },
            });
        }
    }
}

fn reload(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut q_player: Query<(&Player, &mut CanShoot)>,
) {
    for (player, mut can_shoot) in &mut q_player {
        let (input, _) = inputs[player.id];

        // only recharge once we've let go of mouse1
        if can_shoot.value {
            can_shoot.since_last += 1;
        }

        if !input.fire() {
            if !can_shoot.value {
                can_shoot.value = true;
                commands.spawn(AudioBundle {
                    source: asset_server.load("sfx/Bow_Charge.wav"),
                    settings: PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: Volume::Relative(VolumeLevel::new(0.2)),
                        ..default()
                    },
                });
            }
        }
    }
}

fn camera_follow(
    local_player_id: Option<Res<LocalPlayerId>>,
    q_player: Query<(&Player, &Transform)>,
    mut q_camera: Query<
        (&mut Transform, &OrthographicProjection),
        (With<FollowPlayer>, With<Camera>, Without<Player>),
    >,
    q_map: Query<(&Aabb, &Transform), (With<Tilemap>, Without<Camera>, Without<Player>)>,
) {
    let Some(id) = local_player_id else { return; };
    // tilemap aabb relative to itself
    let Ok((map_aabb, map_transform)) = q_map.get_single() else { return; };
    for (player, player_transform) in &q_player {
        if player.id != id.0 {
            continue;
        }

        for (mut transform, proj) in &mut q_camera {
            let player_pos = player_transform.translation.truncate();
            let viewport_area = proj.area;

            let map_center = map_transform
                .transform_point(map_aabb.center.into())
                .truncate();
            let map_halfsize = (map_transform.scale * Vec3::from(map_aabb.half_extents)).truncate();

            // map boundary in world coordinates
            let map_min = map_center - map_halfsize;
            let map_max = map_center + map_halfsize;

            let camera_min = map_min + viewport_area.size() / 2.;
            let camera_max = map_max - viewport_area.size() / 2.;

            transform.translation.x = player_pos.x.clamp(camera_min.x, camera_max.x);
            transform.translation.y = player_pos.y.clamp(camera_min.y, camera_max.y);
        }
    }
}

fn point_bow(
    q_player: Query<&Player>,
    mut q_bow: Query<(&mut Transform, &Parent), With<Bow>>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for (mut transform, parent) in &mut q_bow {
        let Ok(player) = q_player.get(parent.get()) else { continue; };
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
        let Ok(can_shoot) = q_player.get(parent.get()) else { continue; };
        let new_indices = if can_shoot.value { BOW_DRAW } else { BOW_EMPTY };
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

fn move_player(
    mut q_player: Query<(&mut Transform, &mut Velocity, &Player)>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for (mut transform, mut velocity, player) in &mut q_player {
        let (input, _) = inputs[player.id];

        let dir = input.direction();
        let delta = (dir * 1.4).normalize_or_zero();
        velocity.0 = delta;

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
