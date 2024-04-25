use bevy::prelude::*;
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
    dev::config::{DEV_MAX_BUFFER_SIZE, DEV_NETWORK_TICK_DELTA}, 
    netstack::{
        client::Client, 
        components::{
            MinimalNetworkTransform, MinimalNetworkTransformSnapshots, 
            NetClient, NetworkPlayer, NetworkTranslation2D, NetworkYaw, Owner
        }, 
        error::NetstackError, 
        events::{NetworkFireEvent, NetworkMovement2DEvent},
        server::Server
    }
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(PlayerMovementParams{
            base_speed: 10.0,
            prediction_error_threashold: 1.0
        })
        .use_client_event_snapshots::<NetworkMovement2DEvent>(
            ChannelKind::Unreliable, 
            DEV_MAX_BUFFER_SIZE
        )
        .use_component_snapshot::<NetworkTranslation2D>()
        .use_component_snapshot::<NetworkYaw>()
        .add_client_event::<NetworkFireEvent>(ChannelKind::Ordered)
        .replicate::<PlayerPresentation>()
        .replicate::<NetworkTranslation2D>()
        .replicate::<NetworkYaw>()
        .add_systems(FixedUpdate, 
            client_move_2d_system.run_if(resource_exists::<Client>)
        )
        .add_systems(Update, (
            client_on_player_spawned,
            apply_network_transform_system
        ).run_if(resource_exists::<Client>))
        .add_systems(FixedUpdate, 
            server_move_2d_system.run_if(resource_exists::<Server>)
        )
        .add_systems(Update, (
            server_on_player_spawned,
            server_on_fire
        ).run_if(resource_exists::<Server>));
    }
}

pub struct GameIoPlugin;

impl Plugin for GameIoPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_event::<ActionEvent>()
        .add_systems(Update, (
            handle_input_system,
            handle_action_event_system
        ).chain());
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

#[derive(Resource)]
pub struct MouseInputActionMap {
    pub fire: MouseButton
}

#[derive(Event, Default)]
pub struct ActionEvent {
    pub movement_vec: Vec2,
    pub is_fire: bool 
}

impl ActionEvent {
    #[inline]
    pub fn has_movement(&self) -> bool {
        self.movement_vec != Vec2::ZERO
    }
    
    #[inline]
    pub fn has_action(&self) -> bool {
        self.has_movement() || self.is_fire
    }
}

fn handle_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard_action_map: Res<KeyboardInputActionMap>,
    mouse_action_map: Res<MouseInputActionMap>,
    mut actions: EventWriter<ActionEvent> 
) {
    let mut action = ActionEvent::default();
    if keyboard.pressed(keyboard_action_map.movement_up) {
        action.movement_vec.y += 1.0
    } 
    if keyboard.pressed(keyboard_action_map.movement_down) {
        action.movement_vec.y -= 1.0
    }
    if keyboard.pressed(keyboard_action_map.movement_right) {
        action.movement_vec.x += 1.0
    }
    if keyboard.pressed(keyboard_action_map.movement_left) {
        action.movement_vec.x -= 1.0
    }

    if mouse.just_pressed(mouse_action_map.fire) {
        action.is_fire = true;
    }

    if action.has_action() {
        actions.send(action);
    }
} 

fn handle_action_event_system(
    query: Query<(
        &OwnerControlling,
        &ComponentSnapshotBuffer<NetworkTranslation2D>,
        &ComponentSnapshotBuffer<NetworkYaw>
    )>,
    mut actions: EventReader<ActionEvent>,
    mut movements: EventWriter<NetworkMovement2DEvent>,
    mut fires: EventWriter<NetworkFireEvent>
) {
    if let Ok((_, net_t2d_buff, net_yaw_buff)) = query.get_single() {
        for (a, event_id) in actions.read_with_id() {
            if a.has_movement() {
                movements.send(NetworkMovement2DEvent{
                    axis: a.movement_vec,
                    index: event_id.id
                });
            }
            if a.is_fire {
                fires.send(NetworkFireEvent{
                    network_translation_tick: net_t2d_buff.latest_snapshot_tick(),
                    network_yaw_tick: net_yaw_buff.latest_snapshot_tick()
                });
            }
        }
    }
}

fn server_on_player_spawned(
    mut commands: Commands,
    query: Query<(Entity, &NetworkPlayer), Added<NetworkPlayer>>,
    replicon_tick: Res<RepliconTick>
) {
    for (e, p) in query.iter() {
        let tick = replicon_tick.get();
        info!("player: {:?} spawned at tick: {}", p.client_id(), tick);
        
        let mut translation_snaps = ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE);
        // this is for safety pushing older-than-any value
        // other client's latest network tick can be much older than this new client
        // (when they have not moved, synced for a while)
        // this value is catched as latest old value for events from those clients
        translation_snaps.insert(default(), 0);
        translation_snaps.insert(default(), tick);
        let mut rotation_snaps = ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE); 
        rotation_snaps.insert(default(), 0);
        rotation_snaps.insert(default(), tick);

        commands.entity(e)
        .insert((
            MinimalNetworkTransform::default(),
            MinimalNetworkTransformSnapshots {
                translation_snaps,
                rotation_snaps
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
    query: Query<(
        Entity, 
        &NetworkPlayer, &PlayerPresentation, 
        &NetworkTranslation2D, &NetworkYaw
    ), 
        Added<NetworkPlayer>
    >,
    client: Res<Client>,
    server_ticks: Res<ServerEntityTicks>
) {
    for (e, p, presentation, net_t2d, net_yaw) in query.iter() {
        let server_tick = match server_ticks.get(&e) {
            Some(tick) => tick.get(),
            None => {
                if cfg!(debug_assertions) {
                    panic!("server tick is not mapped for this entity: {e:?}");
                } else {
                    warn!("server tick is not mapped for this entity: {e:?}, discarding...");
                    continue;
                }
            }
        };
        info!("player: {:?} spawned at tick: {}", p.client_id(), server_tick);
        
        let mut translation_snaps = ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE);
        // this is for safety pushing older-than-any value
        // other client's latest network tick can be much older than this new client
        // (when they have not moved, synced for a while)
        // this value is catched as latest old value for events from those clients
        translation_snaps.insert(net_t2d.clone(), 0); 
        translation_snaps.insert(net_t2d.clone(), server_tick);
        let mut rotation_snaps = ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE); 
        rotation_snaps.insert(net_yaw.clone(), 0);
        rotation_snaps.insert(net_yaw.clone(), server_tick);

        commands.entity(e)
        .insert((
            PbrBundle{
                mesh: meshes.add(Mesh::from(Capsule3d::default())),
                material: materials.add(presentation.color),
                transform: Transform{
                    translation: net_t2d.to_3d(),
                    rotation: net_yaw.to_3d(),
                    scale: Vec3::ONE
                },
                ..default()
            },
            MinimalNetworkTransformSnapshots {
                translation_snaps,
                rotation_snaps
            },
            EventSnapshotBuffer::<NetworkMovement2DEvent>::new(DEV_MAX_BUFFER_SIZE),
            NetClient::default()
        ));

        if p.client_id().get() == client.id() {
            commands.entity(e).insert(OwnerControlling);
        }
    } 
}

pub fn server_move_2d_system(
    mut query: Query<(&NetworkPlayer, &mut NetworkTranslation2D)>,
    mut movement_snaps: ResMut<EventSnapshotClientMap<NetworkMovement2DEvent>>,
    movement_params: Res<PlayerMovementParams>,
    fixed_time: Res<Time<Fixed>>,
    replicon_tick: Res<RepliconTick>,
) {
    for (net_p, mut net_t2d) in query.iter_mut() {
        let client_id = net_p.client_id();
        
        movement_snaps.sort_with_id(&client_id);
        let frontier = movement_snaps.frontier(&client_id);
        if frontier.len() == 0 {
            continue;
        }
        
        let tick = replicon_tick.get();
        let delta_time = fixed_time.delta_seconds();
        
        let mut t2d = net_t2d.clone();
        for movement in frontier {
            move_2d(&mut t2d, movement.event(), &movement_params, delta_time);
        }
        net_t2d.0 = t2d.0;

        debug!(
            "client: {:?} server translation: {} on tick: {} delta time: {}", 
            client_id, net_t2d.0, tick, delta_time
        );
    }
}

pub fn client_move_2d_system(
    mut query: Query<(
        Entity, 
        &mut Transform,
        &NetworkTranslation2D, 
        &mut EventSnapshotBuffer<NetworkMovement2DEvent>
    ), (
        With<ClientPrediction>, 
        With<OwnerControlling>
    )>,
    movement_params: Res<PlayerMovementParams>,
    server_ticks: Res<ServerEntityTicks>,
    fixed_time: Res<Time<Fixed>>,
    mut errors: EventWriter<NetstackError>
) {
    for (e, mut t, net_t2d, mut movement_buff) in query.iter_mut() {
        let frontier = movement_buff.frontier();
        if frontier.len() == 0 {
            continue;
        }

        let server_tick = match server_ticks.get(&e) {
            Some(tick) => tick.get(),
            None => {
                errors.send(NetstackError(
                    anyhow!("server tick should be stored for this entity: {e:?}")
                ));
                continue;
            }
        };
        let delta_time = fixed_time.delta_seconds();
        
        let mut client_t2d = NetworkTranslation2D::from_3d(t.translation);
        let mut server_t2d = net_t2d.clone();

        for movement in frontier {
            let event = movement.event();
            move_2d(&mut client_t2d, event, &movement_params, delta_time);
            move_2d(&mut server_t2d, event, &movement_params, delta_time);
        }
        debug!("predicted translation: {} on tick {}", client_t2d.0, server_tick);
        debug!("corrected translation: {} on tick {}", server_t2d.0, server_tick);

        let prediction_error = server_t2d.0.distance(client_t2d.0);
        if prediction_error > movement_params.prediction_error_threashold {
            t.translation = server_t2d.to_3d();
            warn!("prediction error(length): {prediction_error} overwritten by server");
        } else {
            t.translation = client_t2d.to_3d();
        }
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

fn apply_network_transform_system(
    mut query: Query<(
        &mut Transform,
        &NetworkTranslation2D, &ComponentSnapshotBuffer<NetworkTranslation2D>, 
        &NetworkYaw, &ComponentSnapshotBuffer<NetworkYaw>
    ), (
        With<InterpolatedReplication>, With<ClientPrediction>, 
        Without<OwnerControlling>
    )>,
    time: Res<Time>
) {
    for (mut t, net_t, net_t_buff, net_y, net_y_buff) in query.iter_mut() {
        let delta_time = time.delta_seconds();
        let mut interpolated_t = net_t.clone();
        interpolate(&mut interpolated_t, net_t_buff, delta_time, DEV_NETWORK_TICK_DELTA);
        let mut interpolated_y = net_y.clone();
        interpolate(&mut interpolated_y, net_y_buff, delta_time, DEV_NETWORK_TICK_DELTA);

        t.translation = interpolated_t.to_3d();
        t.rotation = interpolated_y.to_3d();
    }
}

fn server_on_fire(
    query: Query<(
        &ComponentSnapshotBuffer<NetworkTranslation2D>,
        &ComponentSnapshotBuffer<NetworkYaw>
)   >,
    mut fires: EventReader<FromClient<NetworkFireEvent>>
) {
    for FromClient { client_id, event } in fires.read() {
        info!(
            "player: {client_id:?} fired at it's translation tick: {} yaw tick: {}",
            event.network_translation_tick, 
            event.network_yaw_tick
        );

        for (net_t2d_buff, net_yaw_buff) in query.iter() {
            let net_t2d_idx = match net_t2d_buff.iter()
            .rposition(|s| s.tick() <= event.network_translation_tick) {
                Some(idx) => idx, 
                None => {
                    if cfg!(debug_assertions) {
                        panic!("translation buffer is empty");
                    } else {
                        warn!("translation buffer is empty, ignoring...");
                        return;
                    }
                }
            };
            let net_t2d_snap = net_t2d_buff.get(net_t2d_idx)
            .unwrap(); // must has some here 

            let net_yaw_idx = match net_yaw_buff.iter()
            .rposition(|s| s.tick() <= event.network_yaw_tick) {
                Some(idx) => idx,
                None => {
                    if cfg!(debug_assertions) {
                        panic!("yaw buffer is empty");
                    } else {
                        warn!("yaw buffer is empty, ignoring...");
                        return;
                    }
                }
            };
            let net_yaw_snap = net_yaw_buff.get(net_yaw_idx)
            .unwrap(); // must has some here

            info!(
                "found server transform, translation: {:?} at tick: {}, yaw: {} at tick: {}",
                net_t2d_snap.component().0, net_t2d_snap.tick(),
                net_yaw_snap.component().0, net_yaw_snap.tick(),
            );
        }
    }
}
