use std::net::{IpAddr, Ipv4Addr};
use bevy::prelude::*;
use bevy_net_dev::{
    dev::{
        config::*, 
        game::{GameIoPlugin, GamePlugin, KeyboardInputActionMap, MouseInputActionMap}, 
        level::LevelPlugin
    }, 
    netstack::{
        client::{setup_client, ClientConfig, ClientNetstackPlugin}, 
        error::panic_on_net_error_system
    }
};

fn main() {
    App::new()
    .insert_resource(ClientConfig{
        server_tick_rate: DEV_SERVER_TICK_RATE as u16,
        client_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_port: DEV_SERVER_LISTEN_PORT,
        timeout_seconds: DEV_CLIENT_TIME_OUT_SEC,
        client_id: get_dev_client_id(),
        protocol_id: get_dev_protocol_id(),
        private_key: get_dev_private_key(),
        // I think user data is sent after encryption, am I correct?.
        // https://github.com/mas-bandwidth/netcode/blob/main/STANDARD.md
        user_data: get_dev_user_data(),
        token_expire_seconds: DEV_TOKEN_EXPIRE_SEC,
    })
    .insert_resource(KeyboardInputActionMap{
        movement_up: KeyCode::KeyW,
        movement_left: KeyCode::KeyA,
        movement_down: KeyCode::KeyS,
        movement_right: KeyCode::KeyD,
    })
    .insert_resource(MouseInputActionMap{
        fire: MouseButton::Left
    })
    .add_plugins((
        DefaultPlugins,
        ClientNetstackPlugin
    ))
    .add_plugins((
        GamePlugin,
        GameIoPlugin,
        LevelPlugin
    ))
    // connection to server is not triggered automatically
    .add_systems(Startup, setup_client)
    .add_systems(Update, panic_on_net_error_system)
    .run();
}
