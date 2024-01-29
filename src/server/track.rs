use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{
    binreflect::reflect_to_bin,
    lib_priv::{SyncClientGeneratedEntity, SyncTrackerRes},
    networking::assets::SyncAssetTransfer,
    proto::Message,
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
        let Ok(bin) = reflect_to_bin(change.data.as_reflect(), &registry) else {
            continue;
        };
        let msg = &Message::ComponentUpdated {
            id: change.change_id.id,
            name: change.change_id.name.clone(),
            data: bin,
        };
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(msg).unwrap(),
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
                    continue;
                };
                let AssetId::Uuid { uuid: id } = id else {
                    continue;
                };
                if track.skip_network_handle_change(*id) {
                    continue;
                }
                let Ok(bin) = reflect_to_bin(material.as_reflect(), &registry) else {
                    continue;
                };
                let msg = &Message::StandardMaterialUpdated {
                    id: *id,
                    material: bin,
                };
                for cid in server.clients_id().into_iter() {
                    server.send_message(
                        cid,
                        DefaultChannel::ReliableOrdered,
                        bincode::serialize(msg).unwrap(),
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
    mut sync_assets: ResMut<SyncAssetTransfer>,
) {
    for event in &mut events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(mesh) = assets.get(*id) else {
                    continue;
                };
                let AssetId::Uuid { uuid: id } = id else {
                    continue;
                };
                if track.skip_network_handle_change(*id) {
                    continue;
                }
                let url = sync_assets.serve_mesh(id, mesh);
                for cid in server.clients_id().into_iter() {
                    server.send_message(
                        cid,
                        DefaultChannel::ReliableOrdered,
                        bincode::serialize(&Message::MeshUpdated {
                            id: *id,
                            url: url.clone(),
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

pub(crate) fn react_on_changed_images(
    mut track: ResMut<SyncTrackerRes>,
    mut server: ResMut<RenetServer>,
    assets: Res<Assets<Image>>,
    mut events: EventReader<AssetEvent<Image>>,
    mut sync_assets: ResMut<SyncAssetTransfer>,
) {
    for event in &mut events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(image) = assets.get(*id) else {
                    continue;
                };
                let AssetId::Uuid { uuid: id } = id else {
                    continue;
                };
                if track.skip_network_handle_change(*id) {
                    continue;
                }
                let url = sync_assets.serve_image(id, image);
                for cid in server.clients_id().into_iter() {
                    server.send_message(
                        cid,
                        DefaultChannel::ReliableOrdered,
                        bincode::serialize(&Message::ImageUpdated {
                            id: *id,
                            url: url.clone(),
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
