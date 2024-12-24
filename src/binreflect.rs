use bevy::reflect::{
    serde::{ReflectDeserializer, ReflectSerializer}, PartialReflect, ReflectFromReflect, TypeRegistry
};
use bincode::{DefaultOptions, ErrorKind, Options};
use serde::de::DeserializeSeed;

pub(crate) fn reflect_to_bin(
    compo: &dyn PartialReflect,
    registry: &TypeRegistry,
) -> Result<Vec<u8>, Box<ErrorKind>> {
    let serializer = ReflectSerializer::new(compo, registry);
    DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .serialize(&serializer)
}

pub(crate) fn bin_to_reflect(data: &[u8], registry: &TypeRegistry) -> Box<dyn PartialReflect> {
    let reflect_deserializer = ReflectDeserializer::new(registry);
    let binoptions = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();
    let mut bin_deser = bincode::Deserializer::from_slice(data, binoptions);
    let data = reflect_deserializer.deserialize(&mut bin_deser).unwrap();
    let type_path = data.get_represented_type_info().unwrap().type_path();
    let registration = registry.get_with_type_path(type_path).unwrap();
    let rfr = registry
        .get_type_data::<ReflectFromReflect>(registration.type_id())
        .unwrap();
    rfr.from_reflect(&*data).unwrap().into_partial_reflect()
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy::{
        pbr::OpaqueRendererMethod,
        prelude::*,
        reflect::{GetTypeRegistration, Reflect, ReflectFromReflect, TypeRegistry},
    };
    use serde::{Deserialize, Serialize};

    #[derive(Component, Default, PartialEq, Serialize, Deserialize, Debug, Reflect)]
    struct MyCompo {
        value: i32,
        name: String,
    }

    fn check_serialize_and_back<T>(compo_orig: T)
    where
        T: Reflect + GetTypeRegistration + PartialEq + std::fmt::Debug,
    {
        let mut registry = TypeRegistry::default();
        registry.register::<T>();

        let data = reflect_to_bin(compo_orig.as_partial_reflect(), &registry).unwrap();

        let compo_result = bin_to_reflect(&data, &registry);
        let compo_result = compo_result.try_downcast::<T>().unwrap();

        assert_eq!(*compo_result, compo_orig);
    }

    #[test]
    fn compo_data_serde() {
        check_serialize_and_back::<MyCompo>(MyCompo {
            value: 3,
            name: String::from("name"),
        });
    }

    #[test]
    fn compo_data_serde_bevy_native_component() {
        let compo_orig = Transform::default();

        let mut registry = TypeRegistry::default();
        registry.register::<Transform>();
        registry.register::<Vec3>();
        registry.register::<Quat>();
        registry.register_type_data::<Transform, ReflectFromReflect>();
        registry.register_type_data::<Vec3, ReflectFromReflect>();
        registry.register_type_data::<Quat, ReflectFromReflect>();

        let data = reflect_to_bin(compo_orig.as_partial_reflect(), &registry).unwrap();

        let compo_result = bin_to_reflect(&data, &registry);
        let compo_result = compo_result.try_downcast::<Transform>().unwrap();

        assert_eq!(*compo_result, compo_orig);
    }

    #[test]
    fn material_serde() {
        let material_orig = StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            ..StandardMaterial::default()
        };

        let mut registry = TypeRegistry::default();
        registry.register::<StandardMaterial>();
        registry.register::<Color>();
        registry.register::<Image>();
        registry.register::<Handle<Image>>();
        registry.register::<Option<Handle<Image>>>();
        registry.register::<AlphaMode>();
        registry.register::<ParallaxMappingMethod>();
        registry.register::<OpaqueRendererMethod>();
        registry.register_type_data::<StandardMaterial, ReflectFromReflect>();

        let data = reflect_to_bin(material_orig.as_partial_reflect(), &registry).unwrap();

        let result = bin_to_reflect(&data, &registry);
        let result = result.try_downcast::<StandardMaterial>().unwrap();

        assert_eq!(result.base_color, material_orig.base_color);
    }

    #[test]
    fn reflect_material_no_dependencies() {
        let compo = StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            ..StandardMaterial::default()
        };

        let mut registry = TypeRegistry::default();
        registry.register::<StandardMaterial>();
        registry.register::<Color>();
        registry.register::<Image>();
        registry.register::<Handle<Image>>();
        registry.register::<Option<Handle<Image>>>();
        registry.register::<AlphaMode>();
        registry.register::<ParallaxMappingMethod>();
        registry.register::<OpaqueRendererMethod>();

        // compo_to_bin inlined
        let serializer = ReflectSerializer::new(compo.as_partial_reflect(), &registry);
        let result = DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .serialize(&serializer)
            .unwrap();

        let result = bin_to_reflect(&result, &registry);
        let result = result.try_downcast::<StandardMaterial>().unwrap();
        assert_eq!(compo.base_color, result.base_color);
    }
}
