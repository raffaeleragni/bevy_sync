use bevy::{prelude::Entity, utils::Uuid};
use serde::{Deserialize, Serialize};

pub type EntityId = Entity;
pub type AssId = Uuid;

#[derive(Debug)]
pub(crate) enum SyncAssetType {
    Mesh,
}

#[derive(Serialize, Deserialize, Debug)]
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
        id: AssId,
        material: Vec<u8>,
    } = 6,
    MeshUpdated {
        id: Uuid,
        url: String,
    } = 7,
}
