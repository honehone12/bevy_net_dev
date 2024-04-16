use std::{net::{IpAddr, SocketAddr, UdpSocket}, time::SystemTime};
use bevy::{prelude::*, utils::Uuid};
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig}, 
        ConnectionConfig, RenetServer
    }, 
    RenetChannelsExt, RepliconRenetClientPlugin, RepliconRenetPlugins
};
use bevy_replicon_renet::renet::ClientId as RenetClientId;
use bevy_replicon_snap::RepliconSnapPlugin;
use super::{
    components::{ServerNetworkPlayerInfo, NetworkPlayer}, 
    error::{on_transport_error_system, NetstackError}, 
    resources::{OwnedEntityMap, PlayerEntityMap}
};
use anyhow::anyhow;

#[derive(Resource)]
pub struct ServerParams {
    pub network_tick_rate: u16,
    pub listen_addr: IpAddr,
    pub listen_port: u16,
    pub protocol_id: u64,
    pub private_key: [u8; 32],
    pub max_clients: usize
}

#[derive(Resource)]
pub struct Server;

pub struct ServerNetstackPlugin;

impl Plugin for ServerNetstackPlugin {
    fn build(&self, app: &mut App) {
        let params = app.world.resource::<ServerParams>();
        app.add_plugins((
            RepliconPlugins.build().disable::<ClientPlugin>().set(ServerPlugin{
                tick_policy: TickPolicy::MaxTickRate(params.network_tick_rate),
                ..default()
            }),
            RepliconRenetPlugins.build().disable::<RepliconRenetClientPlugin>(),
            RepliconSnapPlugin
        ))
        .add_event::<NetstackError>()
        .init_resource::<PlayerEntityMap>()
        .init_resource::<OwnedEntityMap>()
        .replicate::<NetworkPlayer>()
        .add_systems(Startup, setup_server)
        .add_systems(Update, (
            handle_server_event_system,
            on_transport_error_system
        ));
    }
}

fn setup_server(
    mut commands: Commands, 
    net_channels: Res<RepliconChannels>,
    params: Res<ServerParams>,
    mut errors: EventWriter<NetstackError>
) {
    let renet_server = RenetServer::new(ConnectionConfig{
        server_channels_config: net_channels.get_server_configs(),
        client_channels_config: net_channels.get_client_configs(),
        ..default()
    });

    let netcode_transport = match setup_transport(&params) {
        Ok(t) => t,
        Err(e) => {
            errors.send(NetstackError(e));
            return;
        }
    };

    info!("server is listening at {}:{}", params.listen_addr, params.listen_port);
    commands.remove_resource::<ServerParams>();
    commands.insert_resource(Server); 
    commands.insert_resource(renet_server);
    commands.insert_resource(netcode_transport);
}

fn setup_transport(params: &ServerParams) 
-> anyhow::Result<NetcodeServerTransport> {
    let listen_addr = SocketAddr::new(params.listen_addr, params.listen_port);
    let socket = UdpSocket::bind(listen_addr)?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let netcode_transport = NetcodeServerTransport::new(ServerConfig{
        current_time,
        max_clients: params.max_clients,
        protocol_id: params.protocol_id,
        authentication: ServerAuthentication::Secure{ 
            private_key: params.private_key
        },
        public_addresses: vec![listen_addr]
    }, socket)?;
    Ok(netcode_transport)
}

fn handle_server_event_system(
    mut commands: Commands,
    mut events: EventReader<ServerEvent>,
    mut palyer_entities: ResMut<PlayerEntityMap>,
    netcode_server: Res<NetcodeServerTransport>, 
    mut errors: EventWriter<NetstackError> 
) {
    for e in events.read() {
        match e {
            ServerEvent::ClientConnected { client_id } => {
                let user_data = match netcode_server.user_data(
                    RenetClientId::from_raw(client_id.get())
                ) {
                    Some(u) => u,
                    None => {
                        errors.send(NetstackError(
                            anyhow!("no user data for this client: {client_id:?}")
                        ));
                        return;
                    }
                };

                let uuid = match Uuid::from_slice(&user_data[0..16]) {
                    Ok(u) => u,
                    Err(e) => {
                        errors.send(NetstackError(e.into()));
                        return;
                    }
                };

                let entity = commands
                    .spawn((
                        ServerNetworkPlayerInfo::new(uuid),
                        NetworkPlayer::new(*client_id)
                    ))
                    .id();
                match palyer_entities.try_insert(*client_id, entity) {
                    Ok(()) => (),
                    Err(e) => {
                        errors.send(NetstackError(e));
                    }
                }                
                info!("client: {client_id:?} id: {uuid} connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                match palyer_entities.get(client_id) {
                    Some(e) => {
                        commands.entity(*e).despawn();
                        palyer_entities.remove(client_id);
                    }
                    None => ()
                }
                info!("client: {client_id:?} disconnected with reason: {reason}");
            }
        }
    }
}
