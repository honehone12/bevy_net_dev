use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct NetworkMovement2DEvent {
    pub axis: Vec2
}
