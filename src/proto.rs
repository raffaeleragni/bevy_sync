use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub(crate) const PROTOCOL_ID: u64 = 1;

type EntityId = Entity;

#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub(crate) enum Message {
    EntitySpawn {
        id: EntityId,
    } = 1,
    EntityParented {
        server_entity_id: EntityId,
        server_parent_id: EntityId,
    } = 2,
    EntitySpawnBack {
        server_entity_id: EntityId,
        client_entity_id: EntityId,
    } = 3,
    EntityDelete {
        id: EntityId,
    } = 4,
    ComponentUpdated {
        id: EntityId,
        name: String,
        data: Vec<u8>,
    } = 5,
}
