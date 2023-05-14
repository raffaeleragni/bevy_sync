use bevy::prelude::Entity;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_ID: u64 = 1;

type SequenceId = u64;
type EntityId = Entity;

#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub enum Message {
    SequenceConfirm { id: SequenceId } = 1,
    SequenceRepeat { id: SequenceId } = 2,
    EntitySpawn { id: EntityId } = 3,
    EntitySpawnBack { id: EntityId, back_id: EntityId } = 4,
    EntityDelete { id: EntityId } = 5,
}
