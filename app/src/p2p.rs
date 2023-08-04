use bevy::prelude::*;
use bevy_ggrs::*;
use bevy_matchbox::prelude::*;

use crate::component::*;
use crate::{input, rand::Rng, GameState};

#[derive(Debug)]
pub struct GgrsConfig;
impl ggrs::Config for GgrsConfig {
    type Input = input::PlayerInput;
    type State = input::PlayerInput;
    type Address = PeerId;
}

#[derive(Resource)]
pub struct LocalPlayerId(pub usize);

pub struct NetworkingPlugin;
impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        GgrsPlugin::<GgrsConfig>::new()
            .with_input_system(input::input)
            .with_update_frequency(60)
            .register_rollback_component::<Transform>()
            .register_rollback_component::<CanShoot>()
            .register_rollback_component::<Velocity>()
            .register_rollback_component::<Lifetime>()
            .register_rollback_component::<InputAngle>()
            .register_rollback_resource::<Rng>()
            .build(app);
    }
}

/// process ggrs events
pub fn process_ggrs_events(
    session: Option<ResMut<Session<GgrsConfig>>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    use ggrs::GGRSEvent;

    let Some(mut session) = session else { return; };
    let Session::P2P(session) = session.as_mut() else { return; };
    for event in session.events() {
        info!("GGRS Event: {event:?}");
        match event {
            GGRSEvent::Disconnected { .. } => {
                warn!("Disconneted. Returning to lobby...");
                next_state.set(GameState::Lobby);
            }
            _ => (),
        }
    }
}

/// initialize the matchbox socket
pub fn setup_socket(mut commands: Commands) {
    let room_url = "ws://sushicat.rocks:9998/p2pg?next=2";
    info!("connecting to room {}", room_url);
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

/// wait for 2 players to connect to the server, before transitioning to in-game
pub fn wait_for_players(
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
        .with_input_delay(2)
        .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 60 });

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
