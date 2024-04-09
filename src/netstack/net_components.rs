use bevy::prelude::*;
use bevy_replicon::core::ClientId;
use bevy_replicon_snap::bevy_replicon_snap_macros::Interpolate;
use serde::{Serialize, Deserialize};

#[derive(Component)]
pub struct NetworkPlayer(ClientId);

impl NetworkPlayer {
    #[inline]
    pub fn new(client_id: ClientId) -> Self {
        Self(client_id)
    }

    #[inline]
    pub fn client_id(&self) -> ClientId {
        self.0
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

#[derive(Component, Interpolate, Serialize, Deserialize)]
pub struct NetworkTranslation2D(pub Vec2);

impl NetworkTranslation2D {
    #[inline]
    pub fn from_3d(vec3: Vec3) -> Self {
        Self(Vec2::new(vec3.x, vec3.z))
    }
    
    #[inline]
    pub fn to_3d(&self) -> Vec3 {
        Vec3::new(self.0.x, 0.0, self.0.y)
    }
}

#[derive(Component, Interpolate, Serialize, Deserialize)]
pub struct NetworkYaw(pub f32);

impl NetworkYaw {
    #[inline]
    pub fn from_3d(quat: Quat) -> Self {
        Self(quat.to_euler(EulerRot::YXZ).0)
    }

    pub fn to_3d(&self) -> Quat {
        Quat::from_rotation_y(self.0.to_radians())
    }
}