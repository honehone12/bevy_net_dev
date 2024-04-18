use bevy::{ecs::query::QuerySingleError, prelude::*};
use bevy_replicon::{
    client::ServerEntityTicks, 
    core::replicon_tick::RepliconTick, 
    prelude::*
};
use bevy_replicon_snap::prelude::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use anyhow::anyhow;
use crate::{
    dev::config::DEV_MAX_BUFFER_SIZE, 
    netstack::{
        client::Client, 
        components::{
            MinimalNetworkTransform, MinimalNetworkTransformSnapshots, NetClient, NetworkPlayer, NetworkTranslation2D, NetworkYaw, Owner
        }, 
        error::NetstackError, 
        events::NetworkMovement2DEvent, 
        resources::{NetworkMovementIndex, PlayerEntityMap}, 
        server::Server
    }
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(PlayerMovementParams{
            base_speed: 10.0,
            prediction_error_threashold: 0.5
        })
        .insert_resource(EventSnapshotHistory::<NetworkMovement2DEvent>::new(DEV_MAX_BUFFER_SIZE))
        .add_client_event::<NetworkMovement2DEvent>(ChannelKind::Unreliable)
        
        .interpolate_replication::<NetworkTranslation2D>()
        .interpolate_replication::<NetworkYaw>()
        
        .add_systems(FixedPreUpdate, (
            client_populate_buffer::<NetworkTranslation2D>,
            client_populate_buffer::<NetworkYaw>
        ).run_if(resource_exists::<Client>))
        .add_systems(FixedPostUpdate, (
            server_populate_buffer::<NetworkTranslation2D>,
            server_populate_buffer::<NetworkYaw>
        ).run_if(resource_exists::<Server>))
        
        .replicate::<PlayerPresentation>()
        .add_systems(FixedUpdate, (
            client_on_player_spawned, 
            client_move_2d_system, 
            apply_transform_presentation
        ).run_if(resource_exists::<Client>))
        .add_systems(FixedUpdate, (
            server_on_player_spawned, 
            server_move_2d_system
        ).run_if(resource_exists::<Server>));
    }
}

pub struct GameIoPlugin;

impl Plugin for GameIoPlugin {
    fn build(&self, app: &mut App) {
        app
        .init_resource::<NetworkMovementIndex>()
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
    mut movements: EventWriter<NetworkMovement2DEvent>,
    mut movement_index: ResMut<NetworkMovementIndex>
) {
    for a in actions.read() {
        if a.has_movement() {
            movements.send(NetworkMovement2DEvent{
                axis: a.movement_vec,
                nonce: movement_index.get()
            });
        }
    }
}

fn server_on_player_spawned(
    mut commands: Commands,
    query: Query<(Entity, &NetworkPlayer), Added<NetworkPlayer>>
) {
    for (e, p) in query.iter() {
        info!("player: {:?} spawned", p.client_id());
        commands.entity(e)
        .insert((
            MinimalNetworkTransform::default(),
            MinimalNetworkTransformSnapshots {
                translation_snaps: ComponentSnapshotBuffer::new(DEV_MAX_BUFFER_SIZE),
                rotation_snap: ComponentSnapshotBuffer::new(DEV_MAX_BUFFER_SIZE)
            },
            Owner::new(p.client_id().get()),
            PlayerPresentation::from_rand_color()
        ));
    }
}

fn client_on_player_spawned(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<
        (Entity, 
            &NetworkPlayer, &PlayerPresentation, 
            &NetworkTranslation2D, &NetworkYaw
        ), 
        Added<NetworkPlayer>
    >,
    client: Res<Client>
) {
    for (e, p, s, t, y) in query.iter() {
        info!("player: {:?} spawned", p.client_id());
        commands.entity(e)
        .insert((
            PbrBundle{
                mesh: meshes.add(Mesh::from(Capsule3d::default())),
                material: materials.add(s.color),
                transform: Transform{
                    translation: t.to_3d(),
                    rotation: y.to_3d(),
                    scale: Vec3::ONE
                },
                ..default()
            },
            MinimalNetworkTransformSnapshots {
                translation_snaps: ComponentSnapshotBuffer::new(DEV_MAX_BUFFER_SIZE),
                rotation_snap: ComponentSnapshotBuffer::new(DEV_MAX_BUFFER_SIZE)
            },
            NetClient::default()
        ));

        if p.client_id().get() == client.id() {
            commands.entity(e).insert(OwnerControlling);
        }
    } 
}

pub fn server_move_2d_system(
    mut query: Query<(&NetworkPlayer, &mut NetworkTranslation2D)>,
    mut movements: EventReader<FromClient<NetworkMovement2DEvent>>,
    mut movement_history: ResMut<EventSnapshotHistory<NetworkMovement2DEvent>>,
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

        let (p, mut net_t2d) = match query.get_mut(*entity) {
            Ok(q) => q,
            Err(e) => {
                errors.send(NetstackError(e.into()));
                continue;
            }
        };

        debug_assert_eq!(p.client_id(), *client_id);
        if let Some(buff) = movement_history.history(client_id.get()) {
            if let Some(snap) = buff.latest_snapshot() {
                if event.nonce <= snap.value().nonce {
                    warn!("discarding a old input, and will affect to replication");
                    continue;
                }
            }
        }
        
        let delta_time = fixed_time.delta_seconds();
        movement_history.insert(client_id.get(), event.clone(), tick.get(), delta_time);
        move_2d(&mut net_t2d, event, &movement_params, delta_time);
        debug!(
            "client: {:?} server translation: {} on tick: {} delta time: {}", 
            client_id, net_t2d.0, tick.get(), delta_time
        );
    }
}

pub fn client_move_2d_system(
    mut query: Query<(
        Entity, 
        &NetworkPlayer, 
        &mut Transform,
        &mut NetworkTranslation2D, 
        &ComponentSnapshotBuffer<NetworkTranslation2D>,
    ), (
        With<ClientPrediction>, 
        With<OwnerControlling>
    )>,
    mut movements: EventReader<NetworkMovement2DEvent>,
    mut movement_history: ResMut<EventSnapshotHistory<NetworkMovement2DEvent>>,
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
    let mut client_t2d = NetworkTranslation2D::from_3d(t.translation);
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
        t.translation = server_t2d.to_3d();
        warn!("client translation is overwritten by server");
    } else {
        t.translation = client_t2d.to_3d();
    }
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
        (With<ClientPrediction>, Without<OwnerControlling>)
    >
) {
    for (net_t, _net_y, mut t) in query.iter_mut() {
        t.translation = net_t.to_3d();
    }
}