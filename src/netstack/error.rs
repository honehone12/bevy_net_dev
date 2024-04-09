use bevy::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;

#[derive(Event)]
pub struct NetstackError {
    pub error: anyhow::Error
}

pub fn panic_on_error_system(mut error: EventReader<NetstackError>) {
    for e in error.read() {
        panic!("ERROR: {}", e.error);
    }
}

pub(crate) fn on_transport_error_system(
    mut netcode_error: EventReader<NetcodeTransportError>,
    mut netstack_error: EventWriter<NetstackError>
) {
    for e in netcode_error.read() {
        netstack_error.send(NetstackError{
            error: anyhow::anyhow!(e.to_string())
        });
    }
} 
