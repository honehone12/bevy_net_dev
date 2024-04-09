use std::net::{IpAddr, Ipv4Addr};
use bevy::prelude::*;
use bevy_net_dev::{
    dev::{dev_config::*, level::LevelPlugin}, 
    netstack::{
        client::{setup_client, ClientNetstackPlugin, ClientParams}, 
        error::panic_on_net_error_system
    }
};

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins((
        LevelPlugin,
        ClientNetstackPlugin{
            network_tick_rate: DEV_NETWORK_TICK_RATE
        },
    ))
    .insert_resource(ClientParams{
        client_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_port: DEV_SERVER_LISTEN_PORT,
        timeout_seconds: DEV_CLIENT_TIME_OUT_SEC,
        protocol_id: get_dev_protocol_id(),
        private_key: get_dev_private_key(),
        user_data: [0; 256],
        token_expire_seconds: DEV_TOKEN_EXPIRE_SEC,
    })
    .add_systems(Startup, setup_client)
    .add_systems(Update, panic_on_net_error_system)
    .run();
}
