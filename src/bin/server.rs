use std::{net::{IpAddr, Ipv4Addr}, time::Duration};
use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use bevy_net_dev::{
    dev::{
        config::*, 
        game::GamePlugin, 
        
    },
    netstack::{ 
        error::panic_on_net_error_system,
        server::{ServerNetstackPlugin, ServerParams}
    }
};
use bevy_replicon_snap::RepliconSnapConfig;

fn main() {
    App::new()
    .insert_resource(ServerParams{
        network_tick_rate: DEV_NETWORK_TICK_RATE,
        listen_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        listen_port: DEV_SERVER_LISTEN_PORT,
        protocol_id: get_dev_protocol_id(),
        private_key: get_dev_private_key(),
        max_clients: DEV_SERVER_MAX_CLIENTS,
    })
    .insert_resource(RepliconSnapConfig{
        max_tick_rate: DEV_NETWORK_TICK_RATE,
        max_buffer_size: DEV_MAX_BUFFER_SIZE,
    })
    .add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f32(DEV_SERVER_TICK_DELTA)
        )),
        LogPlugin::default(),
        ServerNetstackPlugin
    ))
    .add_plugins(GamePlugin)
    .add_systems(Update, panic_on_net_error_system)
    .run();
}
