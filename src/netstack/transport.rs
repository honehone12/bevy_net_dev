use std::net::IpAddr;

#[derive(Clone)]
pub struct TransportParams {
    pub addr: IpAddr,
    pub port: u16,
    pub protocol_id: u64,
    pub private_key: [u8; 32]
}
