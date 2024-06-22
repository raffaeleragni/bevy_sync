use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetServer};
use uuid::Uuid;

use crate::{
    binreflect::reflect_to_bin,
    lib_priv::{SyncClientGeneratedEntity, SyncTrackerRes},
    networking::assets::SyncAssetTransfer,
    proto::Message,
    SyncEntity, SyncMark,
};

pub(crate) fn entity_created_on_server(
    mut track: ResMut<SyncTrackerRes>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    for id in query.iter_mut() {
        let uuid = Uuid::new_v4();
        for client_id in server.clients_id().into_iter() {
            server.send_message(
                client_id,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntitySpawn { id: uuid }).unwrap(),
            );
        }
        let mut entity = commands.entity(id);
        track.uuid_to_entity.insert(uuid, id);
        track.entity_to_uuid.insert(id, uuid);
        entity.remove::<SyncMark>().insert(SyncEntity { uuid });
    }
}

pub(crate) fn entity_parented_on_server(
    mut server: ResMut<RenetServer>,
    track: ResMut<SyncTrackerRes>,
    query: Query<(Entity, &Parent), Changed<Parent>>,
) {
    for (e_id, p) in query.iter() {
        for client_id in server.clients_id().into_iter() {
            let Some(id) = track.entity_to_uuid.get(&e_id) else {
                continue;
            };
            let Some(pid) = track.entity_to_uuid.get(&p.get()) else {
                continue;
            };
            server.send_message(
                client_id,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityParented {
                    entity_id: *id,
                    parent_id: *pid,
                })
                .unwrap(),
            );
        }
    }
}

pub(crate) fn reply_back_to_client_generated_entity(
    mut commands: Commands,
    track: ResMut<SyncTrackerRes>,
    mut server: ResMut<RenetServer>,
    mut query: Query<(Entity, &SyncClientGeneratedEntity), Added<SyncClientGeneratedEntity>>,
) {
    for (entity_id, marker_component) in query.iter_mut() {
        let Some(id) = track.entity_to_uuid.get(&entity_id) else {
            continue;
        };
        for cid in server.clients_id().into_iter() {
            if marker_component.client_id != cid {
                server.send_message(
                    cid,
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::EntitySpawn { id: *id }).unwrap(),
                );
            }
        }
        let mut entity = commands.entity(entity_id);
        entity
            .remove::<SyncClientGeneratedEntity>()
            .insert(SyncEntity { uuid: *id });
    }
}

pub(crate) fn entity_removed_from_server(
    mut server: ResMut<RenetServer>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, With<SyncEntity>>,
) {
    let mut despawned_entities = HashSet::new();
    track.entity_to_uuid.retain(|&e_id, &mut uuid| {
        if query.get(e_id).is_err() {
            despawned_entities.insert(uuid);
            false
        } else {
            true
        }
    });
    for uuid in despawned_entities.iter() {
        track.uuid_to_entity.remove(uuid);
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityDelete { id: *uuid }).unwrap(),
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
        let Some(id) = track.entity_to_uuid.get(&change.change_id.id) else {
            continue;
        };
        let msg = &Message::ComponentUpdated {
            id: *id,
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
