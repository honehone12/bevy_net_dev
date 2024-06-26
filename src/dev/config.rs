use std::time::SystemTime;
use bevy::utils::Uuid;

pub const DEV_SERVER_TICK_RATE: f32 = 20.0;
pub const DEV_SERVER_TICK_DELTA: f32 = 1.0 / DEV_SERVER_TICK_RATE;
pub const DEV_NETWORK_TICK_RATE: u16 = 10;
pub const DEV_NETWORK_TICK_DELTA: f32 = 1.0 / (DEV_NETWORK_TICK_RATE as f32); 

pub const DEV_SERVER_LISTEN_PORT: u16 = 5000;
pub const DEV_SERVER_MAX_CLIENTS: usize = 10;

pub const DEV_CLIENT_TIME_OUT_SEC: i32 = 15;
pub const DEV_TOKEN_EXPIRE_SEC: u64 = 300;

pub const DEV_MAX_BUFFER_SIZE: usize = 100;

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

pub fn get_dev_client_id() -> u64 {
    if cfg!(debug_assertions) {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        now.as_millis() as u64
    } else {
        panic!("do not use dev client id");
    }
}

pub fn get_dev_user_data() -> [u8; 256] {
    if cfg!(debug_assertions) {
        // this will be session id generated by backend service
        let mut user_data = [0u8; 256];
        user_data[0..16].copy_from_slice(Uuid::new_v4().as_bytes());
        user_data
    } else {
        panic!("do not use dev user data")
    }
}

