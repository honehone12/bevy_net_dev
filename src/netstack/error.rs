use bevy::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;

#[derive(Event)]
pub struct NetstackError(pub anyhow::Error);

pub fn panic_on_net_error_system(mut error: EventReader<NetstackError>) {
    for e in error.read() {
        panic!("{}", e.0);
    }
}

pub(crate) fn on_transport_error_system(
    mut netcode_errors: EventReader<NetcodeTransportError>,
    mut netstack_errors: EventWriter<NetstackError>
) {
    for e in netcode_errors.read() {
        netstack_errors.send(NetstackError(anyhow::anyhow!("{e}")));
    }
} 
