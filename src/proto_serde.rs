use std::any::type_name;

use bevy::{
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        DynamicStruct, ReflectFromReflect, TypeRegistryInternal,
    },
};

use bincode::{DefaultOptions, Options};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    ser::{SerializeStruct, Serializer},
    Deserializer, Serialize,
};

pub(crate) fn compo_to_bin(compo: Box<dyn Reflect>, registry: &TypeRegistryInternal) -> Vec<u8> {
    let serializer = ComponentData {
        data: compo.clone_value(),
        registry,
    };
    bincode::serialize(&serializer).unwrap()
}

pub(crate) fn bin_to_compo(data: Vec<u8>, registry: &TypeRegistryInternal) -> Box<dyn Reflect> {
    let binoptions = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();
    let mut bin_deser = bincode::Deserializer::from_slice(&data, binoptions);
    let deserializer = ComponentDataDeserializer { registry };
    let data = deserializer
        .deserialize(&mut bin_deser)
        .unwrap()
        .data
        .downcast::<DynamicStruct>()
        .unwrap();
    let registration = registry.get_with_name(data.name()).unwrap();
    let rfr = registry
        .get_type_data::<ReflectFromReflect>(registration.type_id())
        .unwrap();
    rfr.from_reflect(&*data).unwrap()
}

struct ComponentData<'a> {
    data: Box<dyn Reflect>,
    registry: &'a TypeRegistryInternal,
}

impl<'a> Serialize for ComponentData<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct(type_name::<ComponentData>(), 1)?;
        state.serialize_field(
            "data",
            &ReflectSerializer::new(self.data.as_reflect(), self.registry),
        )?;
        state.end()
    }
}

struct ComponentDataDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'a: 'de, 'de: 'a> DeserializeSeed<'de> for ComponentDataDeserializer<'a> {
    type Value = ComponentData<'a>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let registry = self.registry;
        let data = deserializer.deserialize_struct(type_name::<Self::Value>(), &["data"], self)?;
        Ok(ComponentData { data, registry })
    }
}

impl<'a: 'de, 'de> Visitor<'de> for ComponentDataDeserializer<'a> {
    type Value = Box<dyn Reflect>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(type_name::<Self::Value>())
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        seq.next_element_seed(UntypedReflectDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_type(de::Unexpected::NewtypeVariant, &self))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy::reflect::ReflectFromReflect;
    use serde::Deserialize;

    #[derive(
        Component, Reflect, FromReflect, Default, PartialEq, Serialize, Deserialize, Debug,
    )]
    #[reflect(Component, FromReflect)]
    struct MyCompo {
        value: i32,
        name: String,
    }

    #[test]
    fn compo_data_serde() {
        let compo_orig = MyCompo {
            value: 3,
            name: String::from("name"),
        };

        let mut registry = TypeRegistryInternal::default();
        registry.register::<MyCompo>();

        let data = compo_to_bin(compo_orig.clone_value(), &registry);

        let compo_result = bin_to_compo(data, &registry);
        let compo_result = compo_result.downcast::<MyCompo>().unwrap();

        assert_eq!(*compo_result, compo_orig);
    }
}
