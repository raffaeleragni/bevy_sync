use bevy::{prelude::Entity, reflect::Reflect};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_ID: u64 = 1;

type ComponentType = u64;
type ClientId = u64;

#[derive(Serialize, Deserialize)]
pub enum Message {
    ClientConnected {
        client_id: ClientId,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    ComponentRegistered {
        component_id: ComponentType,
        component_type_name: String,
    },
    ComponentUpdated {
        id: Entity,
        type_id: ComponentType,
        data: Vec<ChangedData>,
    },
    ComponentDestroyed {
        id: Entity,
        type_id: ComponentType,
    },
}

pub struct ChangedData(Box<dyn Reflect>);

impl Serialize for ChangedData {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<'de> Deserialize<'de> for ChangedData {
    fn deserialize<D>(des: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
