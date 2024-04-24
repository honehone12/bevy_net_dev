use bevy::{prelude::*, utils::Uuid};
use bevy_replicon::core::{ClientId, Replication};
use bevy_replicon_snap::prelude::*;
use serde::{Serialize, Deserialize};

// player component each client id has one
#[derive(Component, Serialize, Deserialize)]
pub struct NetworkPlayer {
    client_id: ClientId
}

impl NetworkPlayer {
    #[inline]
    pub fn new(client_id: ClientId) -> Self {
        Self{
            client_id
        }
    }

    #[inline]
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }
}

// component with player info only for server
#[derive(Component)]
pub struct ServerNetworkPlayerInfo {
    uuid: Uuid
}

impl ServerNetworkPlayerInfo {
    #[inline]
    pub fn new(uuid: Uuid) -> Self {
        Self{
            uuid
        }
    }

    #[inline]
    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }
}

// bundle for player controlled entities. each player can have many
#[derive(Bundle)]
pub struct Owner {
    pub owner: NetworkOwner,
    pub replication: Replication,
}

#[derive(Bundle, Default)]
pub struct NetClient {
    pub interpolation: InterpolatedReplication,
    pub prediction: ClientPrediction
}

impl Owner {
    pub fn new(id: u64) -> Self {
        Self { 
            owner: NetworkOwner::new(id), 
            replication: Replication 
        }
    }
}

#[derive(Bundle, Default)]
pub struct MinimalNetworkTransform {
    pub translation: NetworkTranslation2D,
    pub rotation: NetworkYaw,
}

#[derive(Bundle)]
pub struct MinimalNetworkTransformSnapshots {
    pub translation_snaps: ComponentSnapshotBuffer<NetworkTranslation2D>,
    pub rotation_snaps: ComponentSnapshotBuffer<NetworkYaw>
}

#[derive(Component, Serialize, Deserialize, Default, Clone)]
pub struct NetworkTranslation2D(pub Vec2);

impl Interpolate for NetworkTranslation2D {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self(self.0.lerp(other.0, t))
    }
}

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

#[derive(Component, Serialize, Deserialize, Default, Clone)]
pub struct NetworkYaw(pub f32);

impl Interpolate for NetworkYaw {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self(self.0.lerp(other.0, t))
    }
}

impl NetworkYaw {
    #[inline]
    pub fn from_3d(quat: Quat) -> Self {
        Self(quat.to_euler(EulerRot::YXZ).0)
    }

    #[inline]
    pub fn to_3d(&self) -> Quat {
        Quat::from_rotation_y(self.0.to_radians())
    }
}