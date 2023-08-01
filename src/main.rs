use bevy::{
    asset::LoadState,
    prelude::*,
    render::camera::ScalingMode,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_ggrs::*;
use bevy_matchbox::prelude::*;

use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod component;
mod input;
mod map;

use component::*;
use input::{angle_to_vec, from_u8_angle};
use map::*;

struct GgrsConfig;
impl ggrs::Config for GgrsConfig {
    type Input = input::PlayerInput;
    type State = input::PlayerInput;
    type Address = PeerId;
}

#[derive(States, Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum GameState {
    #[default]
    Loading,
    Connecting,
    Countdown,
    Combat,
}

#[derive(Resource)]
struct LoadingAssets(Vec<HandleUntyped>);

#[derive(Resource)]
struct LocalPlayerId(usize);

const MAP_Z: f32 = 0.;
const PLAYER_Z: f32 = 10.;
const BULLET_Z: f32 = 15.;

fn main() {
    let mut app = App::new();

    GgrsPlugin::<GgrsConfig>::new()
        .with_input_system(input::input)
        .register_rollback_component::<Transform>()
        .register_rollback_component::<CanShoot>()
        .build(&mut app);

    app.add_plugins(
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
    .insert_resource(Msaa::Off)
    .add_plugins(map::TiledPlugin)
    .insert_resource(LoadingAssets(vec![]))
    .add_state::<GameState>()
    .add_systems(OnEnter(GameState::Loading), load)
    .add_systems(Update, check_load.run_if(in_state(GameState::Loading)))
    .add_systems(OnExit(GameState::Loading), (setup, setup_socket))
    .add_systems(
        Update,
        wait_for_players.run_if(in_state(GameState::Connecting)),
    )
    .add_systems(OnExit(GameState::Connecting), spawn_players)
    .add_systems(OnEnter(GameState::Countdown), countdown)
    .add_systems(Update, camera_follow)
    .add_systems(
        GgrsSchedule,
        (shoot, move_player, reload, move_bullets)
            .chain()
            .run_if(in_state(GameState::Combat)),
    )
    .add_plugins(WorldInspectorPlugin::new())
    .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    maps: Res<Assets<TiledMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedHorizontal(16. * 20.),
            ..default()
        },
        ..default()
    });

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
        .spawn(MaterialMesh2dBundle {
            mesh: Mesh2dHandle(mesh_handle),
            material: material_handle,
            ..default()
        })
        .insert(
            Transform::from_scale(Vec3::splat(0.5))
                .with_translation((-8. * 25., -8. * 25., MAP_Z).into()),
        );
}

fn load(asset_server: Res<AssetServer>, mut loading: ResMut<LoadingAssets>) {
    let tileset: Handle<Image> = asset_server.load("OverworldTileset_v03.png");
    let tilemap: Handle<TiledMap> = asset_server.load("basic.tmx");

    loading.0.push(tileset.clone_untyped());
    loading.0.push(tilemap.clone_untyped());
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
) {
    for (player, player_transform, mut can_shoot) in &mut q_player {
        let (input, _) = inputs[player.id];

        if input.fire() && can_shoot.0 {
            can_shoot.0 = false;
            commands
                .spawn(Bullet {
                    dir: angle_to_vec(from_u8_angle(input.angle)),
                    vel: 0.64,
                })
                .insert(SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLACK,
                        custom_size: Some(Vec2::splat(5.)),
                        ..default()
                    },
                    ..default()
                })
                .insert(Transform::from_translation(
                    player_transform.translation.truncate().extend(BULLET_Z),
                ))
                .add_rollback();
        }
    }
}

fn reload(inputs: Res<PlayerInputs<GgrsConfig>>, mut q_player: Query<(&Player, &mut CanShoot)>) {
    for (player, mut can_shoot) in &mut q_player {
        let (input, _) = inputs[player.id];

        if !input.fire() {
            can_shoot.0 = true;
        }
    }
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // this will return when the channel has been taken ownership of
    if socket.get_channel(0).is_err() {
        return;
    }

    socket.update_peers();

    let num_players = 2;
    let players = socket.players();
    if players.len() < num_players {
        return;
    }

    info!("All players connected.");

    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new()
        .with_num_players(num_players)
        .with_input_delay(2);

    for (i, player) in players.into_iter().enumerate() {
        if player == ggrs::PlayerType::Local {
            commands.insert_resource(LocalPlayerId(i));
        }
        session_builder = session_builder
            .add_player(player, i)
            .expect("Could not add player to session");
    }

    // give ownership of the channel
    let channel = socket.take_channel(0).unwrap();
    let ggrs_session = session_builder
        .start_p2p_session(channel)
        .expect("Could not init p2p session.");

    commands.insert_resource(Session::P2P(ggrs_session));
    next_state.set(GameState::Countdown);
}

fn setup_socket(mut commands: Commands) {
    let room_url = "ws://192.168.0.149:9998/p2pg?next=2";
    info!("connecting to room {}", room_url);
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

fn camera_follow(
    local_player_id: Option<Res<LocalPlayerId>>,
    q_player: Query<(&Player, &Transform)>,
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let Some(id) = local_player_id else { return; };
    for (player, player_transform) in &q_player {
        if player.id != id.0 {
            continue;
        }

        for mut transform in &mut q_camera {
            transform.translation.x = player_transform.translation.x;
            transform.translation.y = player_transform.translation.y;
        }
    }
}

fn move_player(
    mut q_player: Query<(&mut Transform, &Player)>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
) {
    for (mut transform, player) in &mut q_player {
        let (input, _) = inputs[player.id];

        let dir = input.direction();
        let delta = (dir * 0.8).extend(0.);

        transform.translation += delta;
    }
}

fn spawn_players(mut commands: Commands) {
    commands
        .spawn(Player { id: 0 })
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.2, 0.2, 0.8),
                custom_size: Some(Vec2::new(16., 16.)),
                ..default()
            },
            ..default()
        })
        .insert(Transform::from_xyz(-16., 0., PLAYER_Z))
        .insert(CanShoot(true))
        .add_rollback();
    commands
        .spawn(Player { id: 1 })
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.2, 0.8, 0.2),
                custom_size: Some(Vec2::new(16., 16.)),
                ..default()
            },
            ..default()
        })
        .insert(Transform::from_xyz(16., 0., PLAYER_Z))
        .insert(CanShoot(true))
        .add_rollback();
}
