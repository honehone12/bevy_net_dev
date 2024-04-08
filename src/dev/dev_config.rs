pub const DEV_SERVER_TICK_RATE: f32 = 30.0;
pub const DEV_SERVER_TICK_DELTA: f32 = 1.0 / DEV_SERVER_TICK_RATE;
pub const DEV_NETWORK_TICK_RATE: u16 = 30; 

pub const DEV_SERVER_LISTEN_PORT: u16 = 5000;
pub const DEV_SERVER_MAX_CLIENTS: usize = 10;

pub fn get_dev_protocol_id() -> u64 {
    if cfg!(debug_assertions) {
        0x655ea1eecade99ad
    } else {
        panic!("do not use dev protocol id");
    }
}

pub fn get_dev_private_key() -> [u8; 32] {
    if cfg!(debug_assertions) {
        [
            0x78, 0xe8, 0xbb, 0x30, 0xa2, 0xb, 0x11, 0xf2, 
            0xaa, 0xf6, 0x61, 0x3e, 0xa3, 0xb9, 0xf2, 0x9a, 
            0x53, 0x1f, 0xa7, 0x63, 0x27, 0x27, 0x53, 0x69, 
            0xe4, 0xb2, 0x34, 0x54, 0x15, 0x48, 0x2c, 0xaf
        ]
    } else {
        panic!("do not use dev private key");
    }
}


