use bevy::prelude::*;
use bevy_replicon_snap::snapshots::event_snapshots::IndexedEvent;
use serde::{Serialize, Deserialize};

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct NetworkMovement2DEvent {
    pub axis: Vec2,
    pub index: usize
}

impl IndexedEvent for NetworkMovement2DEvent {
    fn index(&self) -> usize {
        self.index
    }
}
