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
    dev::config::DEV_MAX_BUFFER_SIZE, 
    netstack::{
        client::Client, 
        components::{
            MinimalNetworkTransform, MinimalNetworkTransformSnapshots, 
            NetClient, NetworkPlayer, NetworkTranslation2D, NetworkYaw, Owner
        }, 
        error::NetstackError, 
        events::NetworkMovement2DEvent,
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
        .interpolate_replication::<NetworkTranslation2D>()
        .interpolate_replication::<NetworkYaw>()
        
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
        .add_event::<ActionEvent>()
        .add_systems(Update, (
            handle_keyboard_input_system,
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

fn handle_keyboard_input_system(
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

fn handle_action_event_system(
    mut actions: EventReader<ActionEvent>,
    mut movements: EventWriter<NetworkMovement2DEvent>,
) {
    for (a, event_id) in actions.read_with_id() {
        if a.has_movement() {
            movements.send(NetworkMovement2DEvent{
                axis: a.movement_vec,
                index: event_id.id
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
                translation_snaps: ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE),
                rotation_snap: ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE)
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
                translation_snaps: ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE),
                rotation_snap: ComponentSnapshotBuffer::with_capacity(DEV_MAX_BUFFER_SIZE)
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
        let tick = replicon_tick.get();
        let delta_time = fixed_time.delta_seconds();
        
        movement_snaps.sort_with_id(&client_id);
        let mut t2d = net_t2d.clone();
        for movement in movement_snaps.frontier(&client_id) {
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
        movement_buff.sort_with_id();
        for movement in movement_buff.frontier() {
            move_2d(&mut client_t2d, movement.event(), &movement_params, delta_time);
        }
        debug!("predicted translation: {} on tick {}", client_t2d.0, server_tick);

        let mut server_t2d = net_t2d.clone();
        for movement in movement_buff.frontier() {
            move_2d(&mut server_t2d, movement.event(), &movement_params, delta_time);    
        }
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