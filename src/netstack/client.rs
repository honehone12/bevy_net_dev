use std::{net::{IpAddr, UdpSocket, SocketAddr}, time::SystemTime};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{
        transport::{ClientAuthentication, ConnectToken, NetcodeClientTransport}, 
        ConnectionConfig, RenetClient
    }, 
    RenetChannelsExt, RepliconRenetPlugins, RepliconRenetServerPlugin
};
use bevy_replicon_snap::RepliconSnapPlugin;
use super::{
    components::NetworkPlayer, 
    error::{on_transport_error_system, NetstackError}
};

#[derive(Resource)]
pub struct ClientConfig {
    pub server_tick_rate: u16,
    pub client_addr: IpAddr,
    pub server_addr: IpAddr,
    pub server_port: u16,
    pub timeout_seconds: i32,
    pub client_id: u64,
    pub protocol_id: u64,
    pub private_key: [u8; 32],
    pub user_data: [u8; 256],
    pub token_expire_seconds: u64,
}

#[derive(Resource)]
pub struct Client(u64);

impl Client {
    #[inline]
    pub fn id(&self) -> u64 {
        self.0
    }
}

pub struct ClientNetstackPlugin;

impl Plugin for ClientNetstackPlugin {
    fn build(&self, app: &mut App) {
        let params = app.world.resource::<ClientConfig>();
        app.add_plugins((
            RepliconPlugins.build().disable::<ServerPlugin>(),
            RepliconRenetPlugins.build().disable::<RepliconRenetServerPlugin>(),
            RepliconSnapPlugin{
                server_tick_rate: params.server_tick_rate
            }
        ))
        .add_event::<NetstackError>()
        .replicate::<NetworkPlayer>()
        .add_systems(Update, on_transport_error_system);
    }
}

pub fn setup_client(
    mut commands: Commands,
    net_channels: Res<RepliconChannels>,
    config: Res<ClientConfig>,
    mut errors: EventWriter<NetstackError>
) {
    let renet_client = RenetClient::new(ConnectionConfig{
        server_channels_config: net_channels.get_server_configs(),
        client_channels_config: net_channels.get_client_configs(),
        ..default()
    });

    let netcode_transport = match setup_transport(&config) {
        Ok(t) => t,
        Err(e) => {
            errors.send(NetstackError(e));
            return;
        }
    };
    let client = Client(config.client_id);

    commands.remove_resource::<ClientConfig>();
    commands.insert_resource(client);
    commands.insert_resource(renet_client);
    commands.insert_resource(netcode_transport);
}

fn setup_transport(config: &ClientConfig) 
-> anyhow::Result<NetcodeClientTransport> {
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let socket = UdpSocket::bind((config.client_addr, 0))?;
    let connect_token = ConnectToken::generate(
        current_time,
        config.protocol_id,
        config.token_expire_seconds,
        config.client_id,
        config.timeout_seconds,
        vec![SocketAddr::new(config.server_addr, config.server_port)],
        Some(&config.user_data),
        &config.private_key
    )?;
    let auth = ClientAuthentication::Secure {connect_token};
    let netcode_transport = NetcodeClientTransport::new(current_time, auth, socket)?;
    Ok(netcode_transport)
}
