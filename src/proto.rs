use bevy::{
    asset::AssetIndex,
    prelude::{Asset, AssetId, Entity},
    utils::Uuid,
};
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
        id: AssId,
        material: Vec<u8>,
    } = 6,
    MeshUpdated {
        id: AssId,
        mesh: Vec<u8>,
    } = 7,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum AssId {
    Index { id: AssetIndex },
    Uuid { id: Uuid },
}

impl<T: Asset> From<AssetId<T>> for AssId {
    fn from(value: AssetId<T>) -> Self {
        match value {
            AssetId::Index { index, marker: _ } => AssId::Index { id: index },
            AssetId::Uuid { uuid } => AssId::Uuid { id: uuid },
        }
    }
}
