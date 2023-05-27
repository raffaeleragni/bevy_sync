use std::{any, marker::PhantomData};

use bevy::{
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        TypeRegistryInternal,
    },
};
use bincode::DefaultOptions;
use serde::{
    de::{DeserializeSeed, Visitor},
    ser::{SerializeStruct, Serializer},
    Deserialize, Deserializer, Serialize,
};

pub(crate) const PROTOCOL_ID: u64 = 1;

type SequenceId = u64;
type EntityId = Entity;

#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub(crate) enum Message {
    SequenceConfirm {
        id: SequenceId,
    } = 1,
    SequenceRepeat {
        id: SequenceId,
    } = 2,
    EntitySpawn {
        id: EntityId,
    } = 3,
    EntitySpawnBack {
        server_entity_id: EntityId,
        client_entity_id: EntityId,
    } = 4,
    EntityDelete {
        id: EntityId,
    } = 5,
    EntityComponentUpdated {
        id: EntityId,
        name: String,
        data: Vec<u8>,
    },
}

pub(crate) struct ComponentData<'a> {
    pub(crate) data: Box<dyn Reflect>,
    pub(crate) registry: &'a TypeRegistryInternal,
}

impl<'a> Serialize for ComponentData<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ComponentData", 1)?;
        state.serialize_field(
            "data",
            &ReflectSerializer::new(self.data.as_reflect(), self.registry),
        )?;
        state.end()
    }
}

pub(crate) struct ComponentDataDeserializer<'a> {
    pub(crate) registry: &'a TypeRegistryInternal,
}

impl<'a: 'de, 'de: 'a> DeserializeSeed<'de> for ComponentDataDeserializer<'a> {
    type Value = ComponentData<'a>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisistor<'a> {
            registry: &'a TypeRegistryInternal,
        }
        impl<'a: 'de, 'de> Visitor<'de> for FieldVisistor<'a> {
            type Value = ComponentData<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(any::type_name::<Self::Value>())
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let deser = UntypedReflectDeserializer::new(self.registry);
                let mut bin_deser =
                    bincode::Deserializer::from_slice(&v, DefaultOptions::default());
                let data = deser.deserialize(&mut bin_deser).unwrap();
                Ok(ComponentData {
                    data: data,
                    registry: self.registry,
                })
            }
        }

        deserializer.deserialize_struct(
            "ComponentData",
            &["data"],
            FieldVisistor {
                registry: self.registry,
            },
        )
    }
}
