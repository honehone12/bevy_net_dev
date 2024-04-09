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
use bevy_replicon_snap::SnapshotInterpolationPlugin;
use super::error::{on_transport_error_system, NetstackError};
use anyhow::anyhow;

#[derive(Resource)]
pub struct ClientParams {
    pub client_addr: IpAddr,
    pub server_addr: IpAddr,
    pub server_port: u16,
    pub timeout_seconds: i32,
    pub protocol_id: u64,
    pub private_key: [u8; 32],
    pub user_data: [u8; 256],
    pub token_expire_seconds: u64,
}

#[derive(Resource)]
pub struct Client;

pub struct ClientNetstackPlugin {
    pub network_tick_rate: u16
}

impl Plugin for ClientNetstackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RepliconPlugins.build().disable::<ServerPlugin>(),
            RepliconRenetPlugins.build().disable::<RepliconRenetServerPlugin>(),
            SnapshotInterpolationPlugin{
                max_tick_rate: self.network_tick_rate
            }
        ))
        .add_event::<NetstackError>()
        .add_systems(Update, on_transport_error_system);
    }
}

pub fn setup_client(
    mut commands: Commands,
    net_channels: Res<RepliconChannels>,
    params: Res<ClientParams>,
    mut error: EventWriter<NetstackError>
) {
    let renet_client = RenetClient::new(ConnectionConfig{
        server_channels_config: net_channels.get_server_configs(),
        client_channels_config: net_channels.get_client_configs(),
        ..default()
    });

    let netcode_transport = match setup_transport(&params) {
        Ok(t) => t,
        Err(e) => {
            error.send(NetstackError{
                error: anyhow!(e.to_string())
            });
            return;
        }
    };

    commands.remove_resource::<ClientParams>();
    commands.insert_resource(Client);
    commands.insert_resource(renet_client);
    commands.insert_resource(netcode_transport);
}

fn setup_transport(params: &ClientParams) 
-> anyhow::Result<NetcodeClientTransport> {
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let socket = UdpSocket::bind((params.client_addr, 0))?;
    let connect_token = ConnectToken::generate(
        current_time,
        params.protocol_id,
        params.token_expire_seconds,
        client_id,
        params.timeout_seconds,
        vec![SocketAddr::new(params.server_addr, params.server_port)],
        Some(&params.user_data),
        &params.private_key
    )?;
    let auth = ClientAuthentication::Secure {connect_token};
    let netcode_transport = NetcodeClientTransport::new(current_time, auth, socket)?;
    Ok(netcode_transport)
}