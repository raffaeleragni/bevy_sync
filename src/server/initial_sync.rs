use std::{any::TypeId, error::Error};

use crate::{
    binreflect::reflect_to_bin,
    lib_priv::{SkinnedMeshSyncMapper, SyncTrackerRes},
    networking::assets::SyncAssetTransfer,
    proto::Message,
    SyncEntity,
};
use bevy::{
    prelude::*,
    reflect::DynamicTypePath,
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    utils::HashSet,
};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use uuid::Uuid;

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

fn build_initial_sync(world: &mut World) -> Result<Vec<Message>, Box<dyn Error>> {
    let mut result: Vec<Message> = Vec::new();
    check_entity_components(world, &mut result)?;
    check_parents(world, &mut result)?;
    check_images(world, &mut result)?;
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
        .component_id::<SyncEntity>()
        .ok_or("SyncDown is not registered")?;
    for arch in world
        .archetypes()
        .iter()
        .filter(|arch| arch.contains(sync_down_id))
    {
        for arch_entity in arch.entities() {
            let entity = world.entity(arch_entity.id());
            let e_id = entity.id();
            if let Some(sid) = track.entity_to_uuid.get(&e_id) {
                if !entity_ids_sent.contains(&e_id) {
                    result.push(Message::EntitySpawn { id: *sid });
                    entity_ids_sent.insert(e_id);
                }
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
                let entity = world.entity(arch_entity.id());
                let e_id = entity.id();
                let component = reflect_component.reflect(entity).ok_or("not registered")?;
                let type_name = if component.type_id() == TypeId::of::<SkinnedMesh>() {
                    SkinnedMeshSyncMapper::default()
                        .reflect_type_path()
                        .to_string()
                } else {
                    type_name.to_string()
                };
                let component = if component.type_id() == TypeId::of::<SkinnedMesh>() {
                    debug!("Initial sync: Converting SkinnedMesh to SkinnedMeshSyncMapper");
                    let compo = track
                        .to_skinned_mapper(
                            world.resource::<Assets<SkinnedMeshInverseBindposes>>(),
                            component.downcast_ref::<SkinnedMesh>().unwrap(),
                        )
                        .clone_value();
                    compo
                } else {
                    component.clone_value()
                };
                let compo_bin = match reflect_to_bin(component.as_reflect(), &registry) {
                    Ok(compo_bin) => compo_bin,
                    Err(e) => {
                        debug!(
                            "Initial sync: Could not send component {:?}, {:?}",
                            type_name, e
                        );
                        continue;
                    }
                };
                if let Some(sid) = track.entity_to_uuid.get(&e_id) {
                    result.push(Message::ComponentUpdated {
                        id: *sid,
                        name: type_name,
                        data: compo_bin,
                    });
                }
            }
        }
    }

    Ok(())
}

fn check_parents(world: &World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let track = world.resource::<SyncTrackerRes>();
    let sync_down_id = world
        .component_id::<SyncEntity>()
        .ok_or("SyncDown is not registered")?;
    let parent_component_id = world
        .component_id::<SyncEntity>()
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
                let entity = world.entity(arch_entity.id());
                let e_id = entity.id();
                let Some(parent) = entity.get::<Parent>() else {
                    continue;
                };
                if let Some(sid) = track.entity_to_uuid.get(&e_id) {
                    if let Some(pid) = track.entity_to_uuid.get(&parent.get()) {
                        result.push(Message::EntityParented {
                            entity_id: *sid,
                            parent_id: *pid,
                        });
                    }
                }
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
            let Ok(bin) = reflect_to_bin(material.as_reflect(), &registry) else {
                continue;
            };
            result.push(Message::StandardMaterialUpdated { id, material: bin });
        }
    }
    Ok(())
}

fn check_meshes(world: &mut World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let track = world.resource_mut::<SyncTrackerRes>();
    let mut meshes_to_add = Vec::<(Uuid, Mesh)>::new();
    if track.sync_meshes {
        let meshes = world.resource::<Assets<Mesh>>();
        for (id, mesh) in meshes.iter() {
            let AssetId::Uuid { uuid: id } = id else {
                continue;
            };
            meshes_to_add.push((id, mesh.clone()));
        }
    }
    let mut sync_assets = world.resource_mut::<SyncAssetTransfer>();
    for (id, mesh) in meshes_to_add.iter() {
        let url = sync_assets.serve_mesh(id, mesh);
        result.push(Message::MeshUpdated { id: *id, url });
    }
    Ok(())
}

fn check_images(world: &mut World, result: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
    let track = world.resource_mut::<SyncTrackerRes>();
    let mut images_to_add = Vec::<(Uuid, Image)>::new();
    if track.sync_materials {
        let images = world.resource::<Assets<Image>>();
        for (id, image) in images.iter() {
            let AssetId::Uuid { uuid: id } = id else {
                continue;
            };
            images_to_add.push((id, image.clone()));
        }
    }
    let mut sync_assets = world.resource_mut::<SyncAssetTransfer>();
    for (id, image) in images_to_add.iter() {
        let url = sync_assets.serve_image(id, image);
        result.push(Message::ImageUpdated { id: *id, url });
    }
    Ok(())
}
