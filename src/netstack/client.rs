use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{RepliconRenetPlugins, RepliconRenetServerPlugin};
use bevy_replicon_snap::SnapshotInterpolationPlugin;

#[derive(Clone)]
pub struct ClientParams {

}

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
        ));
    }
}

#[derive(Resource)]
pub struct Client {
    
}

#[derive(Resource)]
pub struct ClientTransport {

}