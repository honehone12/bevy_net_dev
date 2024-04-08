use std::{net::{SocketAddr, UdpSocket}, time::SystemTime};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig}, 
        ConnectionConfig, RenetServer
    }, 
    RenetChannelsExt, RepliconRenetClientPlugin, RepliconRenetPlugins
};
use bevy_replicon_snap::SnapshotInterpolationPlugin;
use crate::netstack::{
    transport::TransportParams,
    error::{NetStackError, on_transport_error_system}
};

#[derive(Clone)]
pub struct ServerParams {
    pub network_tick_rate: u16,
    pub max_clients: usize,
}

pub struct ServerNetstackPlugin {
    pub server_params: ServerParams,
    pub transport_params: TransportParams
}

impl Plugin for ServerNetstackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RepliconPlugins.build().disable::<ClientPlugin>().set(ServerPlugin{
                tick_policy: TickPolicy::MaxTickRate(self.server_params.network_tick_rate),
                ..default()
            }),
            RepliconRenetPlugins.build().disable::<RepliconRenetClientPlugin>(),
            SnapshotInterpolationPlugin{
                max_tick_rate: self.server_params.network_tick_rate
            }
        ))
        .insert_resource(Server{
            params: self.server_params.clone()
        })
        .insert_resource(ServerTransport{
            params: self.transport_params.clone()
        })
        .add_event::<NetStackError>()
        .add_systems(Startup, setup_server)
        .add_systems(Update, on_transport_error_system);
    }
}

#[derive(Resource)]
pub struct Server {
    params: ServerParams
}

impl Server {
    #[inline]
    pub fn get_network_tick_rate(&self) -> u16 {
        self.params.network_tick_rate
    }

    #[inline]
    pub fn get_max_clients(&self) -> usize {
        self.params.max_clients
    }
}

#[derive(Resource)]
struct ServerTransport {
    params: TransportParams
}

fn setup_server(
    mut commands: Commands, 
    net_channels: Res<RepliconChannels>,
    transport: Res<ServerTransport>,
    server: Res<Server>,
    mut error: EventWriter<NetStackError>
) {
    let renet_server = RenetServer::new(ConnectionConfig{
        server_channels_config: net_channels.get_server_configs(),
        client_channels_config: net_channels.get_client_configs(),
        ..default()
    });

    let netcode_transport = match setup_transport(&server, &transport) {
        Ok(t) => t,
        Err(e) => {
            error.send(NetStackError{
                error: anyhow::anyhow!(e.to_string())
            });
            return;
        }
    };

    commands.remove_resource::<ServerTransport>(); 
    commands.insert_resource(renet_server);
    commands.insert_resource(netcode_transport);
}

fn setup_transport(server: &Server, transport: &ServerTransport) 
-> anyhow::Result<NetcodeServerTransport> {
    let listen_addr = SocketAddr::new(transport.params.addr, transport.params.port);
    let socket = UdpSocket::bind(listen_addr)?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let netcode_transport = NetcodeServerTransport::new(ServerConfig{
        current_time,
        max_clients: server.params.max_clients,
        protocol_id: transport.params.protocol_id,
        authentication: ServerAuthentication::Secure{ 
            private_key: transport.params.private_key
        },
        public_addresses: vec![listen_addr]
    }, socket)?;
    Ok(netcode_transport)
}
