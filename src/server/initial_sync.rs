use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{lib_priv::SyncTrackerRes, proto::Message, proto_serde::compo_to_bin, SyncDown};

pub(crate) fn send_initial_sync(client_id: u64, world: &mut World) {
    info!("Sending initial sync to client id: {}", client_id);
    // exclusive access to world while looping through all objects, this can be blocking/freezing for the server
    let mut initial_sync = build_initial_sync(world);
    let mut server = world.resource_mut::<RenetServer>();
    debug!("Initial sync size: {}", initial_sync.len());
    for msg in initial_sync.drain(..) {
        let msg_bin = bincode::serialize(&msg).unwrap();
        server.send_message(client_id, DefaultChannel::ReliableOrdered, msg_bin);
    }
}

pub(crate) fn build_initial_sync(world: &World) -> Vec<Message> {
    let mut entity_ids_sent: HashSet<Entity> = HashSet::new();
    let mut result: Vec<Message> = Vec::new();
    let track = world.resource::<SyncTrackerRes>();
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let sync_down_id = world
        .component_id::<SyncDown>()
        .expect("SyncDown is not registered");
    for arch in world
        .archetypes()
        .iter()
        .filter(|arch| arch.contains(sync_down_id))
    {
        for arch_entity in arch.entities() {
            let entity = world.entity(arch_entity.entity());
            let e_id = entity.id();
            if !entity_ids_sent.contains(&e_id) {
                result.push(Message::EntitySpawn { id: e_id });
                entity_ids_sent.insert(e_id);
            }
        }
        for c_id in arch
            .components()
            .filter(|&c_id| track.sync_components.contains(&c_id))
        {
            let c_exclude_id = track
                .exclude_components
                .get(&c_id)
                .expect("Sync component not setup correctly, missing SyncExclude<T>");
            if arch.contains(*c_exclude_id) {
                continue;
            }
            let c_info = world
                .components()
                .get_info(c_id)
                .expect("component not found");
            let type_name = c_info.name();
            let registration = registry
                .get(c_info.type_id().expect("not registered"))
                .expect("not registered");
            let reflect_component = registration
                .data::<ReflectComponent>()
                .expect("missing #[reflect(Component)]");
            for arch_entity in arch.entities() {
                let entity = world.entity(arch_entity.entity());
                let e_id = entity.id();
                let component = reflect_component.reflect(entity).expect("not registered");
                let compo_bin = compo_to_bin(component.clone_value(), &registry);
                result.push(Message::ComponentUpdated {
                    id: e_id,
                    name: type_name.into(),
                    data: compo_bin,
                });
            }
        }
    }

    if track.sync_materials_enabled() {
        let materials = world.resource::<Assets<StandardMaterial>>();
        for (id, material) in materials.iter() {
            result.push(Message::StandardMaterialUpdated {
                id,
                material: compo_to_bin(material.clone_value(), &registry),
            });
        }
    }

    result
}
