use bevy::prelude::*;
use bevy_net_dev::{
    dev::{dev_config::DEV_NETWORK_TICK_RATE, level::LevelPlugin}, 
    netstack::client::ClientNetstackPlugin
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
    .run();
}
