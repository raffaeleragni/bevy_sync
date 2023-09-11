use bevy::{asset::HandleId, prelude::Entity};
use serde::{Deserialize, Serialize};

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
    StandardMaterialUpdated {
        id: HandleId,
        material: Vec<u8>,
    } = 6,
    MeshUpdated {
        id: HandleId,
        mesh: Vec<u8>,
    } = 7,
}
