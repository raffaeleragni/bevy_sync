use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{
    lib_priv::{SyncClientGeneratedEntity, SyncTrackerRes},
    mesh_serde::mesh_to_bin,
    proto::Message,
    proto_serde::compo_to_bin,
    SyncDown, SyncMark,
};

pub(crate) fn track_spawn_server(
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, Added<SyncDown>>,
) {
    for e_id in query.iter() {
        track.server_to_client_entities.insert(e_id, e_id);
    }
}

pub(crate) fn entity_created_on_server(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    for id in query.iter_mut() {
        debug!(
            "New entity created on server: {}v{}",
            id.index(),
            id.generation()
        );
        for client_id in server.clients_id().into_iter() {
            server.send_message(
                client_id,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
            );
        }
        let mut entity = commands.entity(id);
        entity.remove::<SyncMark>().insert(SyncDown {});
    }
}

pub(crate) fn entity_parented_on_server(
    mut server: ResMut<RenetServer>,
    query: Query<(Entity, &Parent), Changed<Parent>>,
) {
    for (e_id, p) in query.iter() {
        for client_id in server.clients_id().into_iter() {
            server.send_message(
                client_id,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityParented {
                    server_entity_id: e_id,
                    server_parent_id: p.get(),
                })
                .unwrap(),
            );
        }
    }
}

pub(crate) fn reply_back_to_client_generated_entity(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut query: Query<(Entity, &SyncClientGeneratedEntity), Added<SyncClientGeneratedEntity>>,
) {
    for (entity_id, marker_component) in query.iter_mut() {
        debug!(
            "Replying to client generated entity for: {}v{}",
            entity_id.index(),
            entity_id.generation()
        );
        server.send_message(
            marker_component.client_id,
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntitySpawnBack {
                server_entity_id: entity_id,
                client_entity_id: marker_component.client_entity_id,
            })
            .unwrap(),
        );
        for cid in server.clients_id().into_iter() {
            if marker_component.client_id != cid {
                server.send_message(
                    cid,
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::EntitySpawn { id: entity_id }).unwrap(),
                );
            }
        }
        let mut entity = commands.entity(entity_id);
        entity
            .remove::<SyncClientGeneratedEntity>()
            .insert(SyncDown {});
    }
}

pub(crate) fn entity_removed_from_server(
    mut server: ResMut<RenetServer>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, With<SyncDown>>,
) {
    let mut despawned_entities = HashSet::new();
    track.server_to_client_entities.retain(|&e_id, _| {
        if query.get(e_id).is_err() {
            despawned_entities.insert(e_id);
            false
        } else {
            true
        }
    });
    for &id in despawned_entities.iter() {
        debug!(
            "Entity was removed from server: {}v{}",
            id.index(),
            id.generation()
        );
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityDelete { id }).unwrap(),
            );
        }
    }
}

pub(crate) fn react_on_changed_components(
    registry: Res<AppTypeRegistry>,
    mut server: ResMut<RenetServer>,
    mut track: ResMut<SyncTrackerRes>,
) {
    let registry = registry.read();
    while let Some(change) = track.changed_components_to_send.pop_front() {
        debug!(
            "Component was changed on server: {}",
            change.data.get_represented_type_info().unwrap().type_path()
        );
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::ComponentUpdated {
                    id: change.change_id.id,
                    name: change.change_id.name.clone(),
                    data: compo_to_bin(change.data.as_reflect(), &registry),
                })
                .unwrap(),
            );
        }
    }
}

pub(crate) fn react_on_changed_materials(
    mut track: ResMut<SyncTrackerRes>,
    registry: Res<AppTypeRegistry>,
    mut server: ResMut<RenetServer>,
    materials: Res<Assets<StandardMaterial>>,
    mut events: EventReader<AssetEvent<StandardMaterial>>,
) {
    let registry = registry.read();
    for event in &mut events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(material) = materials.get(*id) else {
                    return;
                };
                let AssetId::Uuid { uuid: id } = id else {
                    return;
                };
                if track.skip_network_handle_change(*id) {
                    return;
                }
                for cid in server.clients_id().into_iter() {
                    server.send_message(
                        cid,
                        DefaultChannel::ReliableOrdered,
                        bincode::serialize(&Message::StandardMaterialUpdated {
                            id: *id,
                            material: compo_to_bin(material.as_reflect(), &registry),
                        })
                        .unwrap(),
                    );
                }
            }
            AssetEvent::Removed { id: _ } => {}
            _ => (),
        }
    }
}

pub(crate) fn react_on_changed_meshes(
    mut track: ResMut<SyncTrackerRes>,
    mut server: ResMut<RenetServer>,
    assets: Res<Assets<Mesh>>,
    mut events: EventReader<AssetEvent<Mesh>>,
) {
    for event in &mut events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(mesh) = assets.get(*id) else {
                    return;
                };
                let AssetId::Uuid { uuid: id } = id else {
                    return;
                };
                if track.skip_network_handle_change(*id) {
                    return;
                }
                for cid in server.clients_id().into_iter() {
                    server.send_message(
                        cid,
                        DefaultChannel::ReliableOrdered,
                        bincode::serialize(&Message::MeshUpdated {
                            id: *id,
                            mesh: mesh_to_bin(mesh),
                        })
                        .unwrap(),
                    );
                }
            }
            AssetEvent::Removed { id: _ } => {}
            _ => (),
        }
    }
}
