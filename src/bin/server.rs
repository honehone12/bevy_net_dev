use std::{net::{IpAddr, Ipv4Addr}, time::Duration};
use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use bevy_net_dev::{
    dev::dev_config::*, 
    netstack::{
        transport::TransportParams, 
        server::{ServerNetstackPlugin, ServerParams},
        error::panic_on_error_system
    }
};

fn main() {
    App::new()
    .add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f32(DEV_SERVER_TICK_DELTA)
        )),
        LogPlugin::default()
    ))
    .add_plugins(ServerNetstackPlugin{
        server_params: ServerParams{
            network_tick_rate: DEV_NETWORK_TICK_RATE,
            max_clients: DEV_SERVER_MAX_CLIENTS,
        },
        transport_params: TransportParams{
            addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEV_SERVER_LISTEN_PORT,
            protocol_id: get_dev_protocol_id(),
            private_key: get_dev_private_key(),
        }
    })
    .add_systems(Update, panic_on_error_system)
    .run();
}
