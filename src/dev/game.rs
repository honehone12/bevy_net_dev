use bevy::{ecs::query::QuerySingleError, prelude::*};
use bevy_replicon::{client::ServerEntityTicks, core::replicon_tick::RepliconTick, prelude::*};
use bevy_replicon_snap::{
    RepliconSnapExt, ComponentSnapshotBuffer, Interpolated, Predicted, PredictedEventHistory
};
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use anyhow::anyhow;
use crate::netstack::{
    client::Client, 
    components::{
        MinimalNetworkTransform, NetworkPlayer, NetworkTranslation2D, NetworkYaw, OwnerControlled
    }, 
    error::NetstackError, 
    events::NetworkMovement2DEvent, 
    resources::PlayerEntityMap, 
    server::Server
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(PlayerMovementParams{
            base_speed: 10.0,
            prediction_error_threashold: 0.5
        })
        .replicate_interpolated::<NetworkTranslation2D>()
        .replicate_interpolated::<NetworkYaw>()
        .replicate::<PlayerPresentation>()
        .add_client_predicted_event::<NetworkMovement2DEvent>(ChannelKind::Unreliable)
        .add_systems(FixedUpdate, (
            on_player_spawned_client_system, 
            client_move_2d, 
            apply_transform_presentation
        ).run_if(resource_exists::<Client>))
        .add_systems(FixedUpdate, (
            on_player_spawned_server_system, 
            server_move_2d
        ).run_if(resource_exists::<Server>));
    }
}

pub struct GameIoPlugin;

impl Plugin for GameIoPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_event::<ActionEvent>()
        .add_systems(Update, (
            handle_keyboard_input_system,
            handle_action_event_system
        ));
    }
}

#[derive(Resource)]
pub struct PlayerMovementParams {
    pub base_speed: f32,
    pub prediction_error_threashold: f32
}

#[derive(Component, Serialize, Deserialize)]
pub struct PlayerPresentation {
    pub color: Color
}

impl PlayerPresentation {
    #[inline]
    pub fn from_rand_color() -> Self {
        Self{
            color: Color::rgb(
                random(), 
                random(), 
                random()
            )
        }
    }
}

#[derive(Resource)]
pub struct KeyboardInputActionMap {
    pub movement_up: KeyCode,
    pub movement_left: KeyCode,
    pub movement_down: KeyCode,
    pub movement_right: KeyCode
}

#[derive(Event, Default)]
pub struct ActionEvent {
    pub movement_vec: Vec2
}

impl ActionEvent {
    #[inline]
    pub fn has_movement(&self) -> bool {
        self.movement_vec != Vec2::ZERO
    }
    
    #[inline]
    pub fn has_action(&self) -> bool {
        // will have has_fire, has_jump or something else
        self.has_movement()
    }
}

pub fn handle_keyboard_input_system(
    key_board: Res<ButtonInput<KeyCode>>,
    action_map: Res<KeyboardInputActionMap>,
    mut actions: EventWriter<ActionEvent> 
) {
    let mut action = ActionEvent::default();
    if key_board.pressed(action_map.movement_up) {
        action.movement_vec.y += 1.0
    } 
    if key_board.pressed(action_map.movement_down) {
        action.movement_vec.y -= 1.0
    }
    if key_board.pressed(action_map.movement_right) {
        action.movement_vec.x += 1.0
    }
    if key_board.pressed(action_map.movement_left) {
        action.movement_vec.x -= 1.0
    }

    if action.has_action() {
        actions.send(action);
    }
} 

pub fn handle_action_event_system(
    mut actions: EventReader<ActionEvent>,
    mut movements: EventWriter<NetworkMovement2DEvent>
) {
    for a in actions.read() {
        if a.has_movement() {
            movements.send(NetworkMovement2DEvent{
                axis: a.movement_vec
            });
        }
    }
}

fn on_player_spawned_server_system(
    mut commands: Commands,
    query: Query<(Entity, &NetworkPlayer), Added<NetworkPlayer>>
) {
    for (e, p) in query.iter() {
        info!("player: {:?} spawned, inserting network transform...", p.client_id());
        commands.entity(e)
        .insert((
            MinimalNetworkTransform::default(), 
            OwnerControlled::new(p.client_id().get()),
            PlayerPresentation::from_rand_color()
        ))
        .log_components();
    }
}

fn on_player_spawned_client_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<
        (Entity, 
            &NetworkPlayer, &PlayerPresentation, 
            &NetworkTranslation2D, &NetworkYaw
        ), 
        Added<NetworkPlayer>
    >
) {
    for (e, p, s, t, y) in query.iter() {
        info!("player: {:?} spawned, inserting visual components...", p.client_id());
        commands.entity(e)
        .insert(
            PbrBundle{
                mesh: meshes.add(Mesh::from(Capsule3d::default())),
                material: materials.add(s.color),
                transform: Transform{
                    translation: t.to_3d(),
                    rotation: y.to_3d(),
                    scale: Vec3::ONE
                },
                ..default()
            }
        )
        .log_components();
    } 
}

pub fn server_move_2d(
    mut query: Query<(&NetworkPlayer, &mut NetworkTranslation2D)>,
    mut movements: EventReader<FromClient<NetworkMovement2DEvent>>,
    mut movement_history: ResMut<PredictedEventHistory<NetworkMovement2DEvent>>,
    movement_params: Res<PlayerMovementParams>,
    player_entities: Res<PlayerEntityMap>,
    fixed_time: Res<Time<Fixed>>,
    tick: Res<RepliconTick>,
    mut errors: EventWriter<NetstackError>
) {
    for FromClient { client_id, event } in movements.read() {
        let entity = match player_entities.get(client_id) {
            Some(e) => e,
            None => {
                errors.send(NetstackError(
                    anyhow!("client: {client_id:?} is not mapped for it's player entity")
                ));
                continue;
            }
        };

        match query.get_mut(*entity) {
            Ok((p, mut net_t2d)) => {
                debug_assert_eq!(p.client_id(), *client_id);
                let delta_time = fixed_time.delta_seconds();
                movement_history.insert(client_id.get(), event.clone(), tick.get(), delta_time);
                move_2d(&mut net_t2d, event, &movement_params, delta_time);
                debug!(
                    "client: {:?} server translation: {} on tick: {} delta time: {}", 
                    client_id, net_t2d.0, tick.get(), delta_time
                );
            }
            Err(e) => {
                errors.send(NetstackError(e.into()));
                continue;
            }
        }
    }
}

pub fn client_move_2d(
    mut query: Query<(
        Entity, 
        &NetworkPlayer, 
        &mut Transform,
        &mut NetworkTranslation2D, 
        &ComponentSnapshotBuffer<NetworkTranslation2D>,
    ), (
        With<Predicted>, 
        Without<Interpolated>
    )>,
    mut movements: EventReader<NetworkMovement2DEvent>,
    mut movement_history: ResMut<PredictedEventHistory<NetworkMovement2DEvent>>,
    movement_params: Res<PlayerMovementParams>,
    server_ticks: Res<ServerEntityTicks>,
    fixed_time: Res<Time<Fixed>>,
    mut errors: EventWriter<NetstackError>
) {
    let (e, net_p, mut t, mut net_t2d, net_t2d_buff) = match query.get_single_mut() {
        Ok((e, p, t, nt, b)) => (e, p, t, nt, b),
        Err(QuerySingleError::NoEntities(_)) => {
            return;
        }
        Err(QuerySingleError::MultipleEntities(e)) => {
            errors.send(NetstackError(anyhow!(e)));
            return;
        }
    };

    let server_tick = match server_ticks.get(&e) {
        Some(tick) => tick.get(),
        None => {
            errors.send(NetstackError(
                anyhow!("server tick should be stored for this entity: {e:?}")
            ));
            return;
        }
    };

    let client_id = net_p.client_id().get();
    let delta_time = fixed_time.delta_seconds();
    let mut client_t2d = net_t2d.clone();
    for m in movements.read() {
        movement_history.insert(client_id, m.clone(), server_tick, delta_time);
        move_2d(&mut client_t2d, m, &movement_params, delta_time);
        debug!("predicted translation: {}", client_t2d.0);
    }

    let latest_snapshot = match net_t2d_buff.latest_snapshot() {
        Some(s) => s,
        None => {
            net_t2d.0 = client_t2d.0;
            t.translation = net_t2d.to_3d();
            return;
        }
    };

    let mut server_t2d = latest_snapshot.value().clone();
    for m in movement_history.frontier(client_id, latest_snapshot.tick()) {
        move_2d(&mut server_t2d, m.value(), &movement_params, m.delta_time());
    }
    
    debug!("corrected translation: {}", server_t2d.0);
    let prediction_error = server_t2d.0.distance(client_t2d.0);
    debug!("prediction error(length): {prediction_error}");
    if prediction_error > movement_params.prediction_error_threashold {
        net_t2d.0 = server_t2d.0;
        warn!("client translation is overwritten by server");
    } else {
        net_t2d.0 = client_t2d.0;
    }

    t.translation = net_t2d.to_3d();
}

fn move_2d(
    translation: &mut NetworkTranslation2D,
    movement: &NetworkMovement2DEvent,
    params: &PlayerMovementParams,
    delta_time: f32
) {
    let mut dir = movement.axis.normalize();
    dir.y *= -1.0;
    translation.0 += dir * (params.base_speed * delta_time); 
}

fn apply_transform_presentation(
    mut query: Query<
        (&NetworkTranslation2D, &NetworkYaw, &mut Transform),
        (With<Interpolated>, Without<Predicted>)
    >
) {
    for (net_t, _net_y, mut t) in query.iter_mut() {
        t.translation = net_t.to_3d();
    }
}