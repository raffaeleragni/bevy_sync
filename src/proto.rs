use bevy::ecs::event::Event;
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::SyncConnectionParameters;

pub type EntityId = Uuid;
pub type AssId = Uuid;

#[derive(Debug)]
pub(crate) enum SyncAssetType {
    Mesh,
    Image,
    Audio,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub(crate) enum Message {
    EntitySpawn {
        id: EntityId,
    } = 1,
    EntityParented {
        entity_id: EntityId,
        parent_id: EntityId,
    } = 2,
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
    ImageUpdated {
        id: Uuid,
        url: String,
    } = 8,
    AudioUpdated {
        id: Uuid,
        url: String,
    } = 9,
    PromoteToHost = 10,
    NewHost {
        params: SyncConnectionParameters,
    } = 11,
    RequestInitialSync = 12,
    FinishedInitialSync = 13,
}

#[derive(Event)]
pub struct PromoteToHostEvent {
    pub id: ClientId,
}
