use std::error::Error;

use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};

use crate::{
    lib_priv::SyncTrackerRes, mesh_serde::mesh_to_bin, proto::Message, proto_serde::compo_to_bin,
    SyncDown,
};

pub(crate) fn send_initial_sync(client_id: ClientId, world: &mut World) {
    info!("Sending initial sync to client id {}", client_id);
    // exclusive access to world while looping through all objects, this can be blocking/freezing for the server
    let mut initial_sync = match build_initial_sync(world) {
        Ok(initial_sync) => initial_sync,
        Err(err) => {
            warn!(
                "Failed initial sync to client id {} because {}",
                client_id, err
            );
            return;
        }
    };
    let mut server = world.resource_mut::<RenetServer>();
    debug!("Initial sync size: {}", initial_sync.len());
    for msg in initial_sync.drain(..) {
        let Ok(msg_bin) = bincode::serialize(&msg) else {
            warn!("Could not deserialize {:?}", msg);
            continue;
        };
        server.send_message(client_id, DefaultChannel::ReliableOrdered, msg_bin);
    }
}

fn build_initial_sync(world: &World) -> Result<Vec<Message>, Box<dyn Error>> {
    let mut result: Vec<Message> = Vec::new();
    check_entity_components(world, &mut result)?;
    check_parents(world, &mut result)?;
    check_materials(world, &mut result)?;
    check_meshes(world, &mut result)?;
    Ok(result)
}

fn check_entity_components(world: &World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let mut entity_ids_sent: HashSet<Entity> = HashSet::new();
    let track = world.resource::<SyncTrackerRes>();
    let registry = world.resource::<AppTypeRegistry>();
    let registry = registry.read();
    let sync_down_id = world
        .component_id::<SyncDown>()
        .ok_or("SyncDown is not registered")?;
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
            .filter(|&c_id| track.registered_componets_for_sync.contains(&c_id))
        {
            let c_exclude_id = track
                .sync_exclude_cid_of_component_cid
                .get(&c_id)
                .ok_or("Sync component not setup correctly, missing SyncExclude<T>")?;
            if arch.contains(*c_exclude_id) {
                continue;
            }
            let c_info = world
                .components()
                .get_info(c_id)
                .ok_or("component not found")?;
            let type_name = c_info.name();
            let registration = registry
                .get(c_info.type_id().ok_or("not registered")?)
                .ok_or("not registered")?;
            let reflect_component = registration
                .data::<ReflectComponent>()
                .ok_or("missing #[reflect(Component)]")?;
            for arch_entity in arch.entities() {
                let entity = world.entity(arch_entity.entity());
                let e_id = entity.id();
                let component = reflect_component.reflect(entity).ok_or("not registered")?;
                let Ok(compo_bin) = compo_to_bin(component.as_reflect(), &registry) else {break};
                result.push(Message::ComponentUpdated {
                    id: e_id,
                    name: type_name.into(),
                    data: compo_bin,
                });
            }
        }
    }

    Ok(())
}

fn check_parents(world: &World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let sync_down_id = world
        .component_id::<SyncDown>()
        .ok_or("SyncDown is not registered")?;
    let parent_component_id = world
        .component_id::<SyncDown>()
        .ok_or("Parent is not registered")?;
    for arch in world
        .archetypes()
        .iter()
        .filter(|arch| arch.contains(sync_down_id))
    {
        for _ in arch
            .components()
            .filter(|&c_id| c_id == parent_component_id)
        {
            for arch_entity in arch.entities() {
                let entity = world.entity(arch_entity.entity());
                let e_id = entity.id();
                let Some(parent) = entity.get::<Parent>() else {
                    continue;
                };
                result.push(Message::EntityParented {
                    server_entity_id: e_id,
                    server_parent_id: parent.get(),
                });
            }
        }
    }
    Ok(())
}

fn check_materials(world: &World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let track = world.resource::<SyncTrackerRes>();
    let registry = world.resource::<AppTypeRegistry>();
    let registry = registry.read();
    if track.sync_materials {
        let materials = world.resource::<Assets<StandardMaterial>>();
        for (id, material) in materials.iter() {
            let AssetId::Uuid { uuid: id } = id else {
                continue;
            };
            let Ok(bin) = compo_to_bin(material.as_reflect(), &registry) else {
                break;
            };
            result.push(Message::StandardMaterialUpdated { id, material: bin });
        }
    }
    Ok(())
}

fn check_meshes(world: &World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let track = world.resource::<SyncTrackerRes>();
    if track.sync_meshes {
        let meshes = world.resource::<Assets<Mesh>>();
        for (id, mesh) in meshes.iter() {
            let AssetId::Uuid { uuid: id } = id else {
                continue;
            };
            result.push(Message::MeshUpdated {
                id,
                mesh: mesh_to_bin(mesh),
            });
        }
    }
    Ok(())
}
