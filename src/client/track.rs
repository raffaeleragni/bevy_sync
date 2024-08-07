use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetClient};
use uuid::Uuid;

use crate::{
    binreflect::reflect_to_bin, lib_priv::SyncTrackerRes, networking::assets::SyncAssetTransfer,
    proto::Message, SyncEntity, SyncMark,
};

pub(crate) fn entity_created_on_client(
    mut track: ResMut<SyncTrackerRes>,
    mut client: ResMut<RenetClient>,
    mut query: Query<Entity, Added<SyncMark>>,
    mut cmd: Commands,
) {
    for id in query.iter_mut() {
        let uuid = Uuid::new_v4();
        track.uuid_to_entity.insert(uuid, id);
        track.entity_to_uuid.insert(id, uuid);
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntitySpawn { id: uuid }).unwrap(),
        );
        cmd.entity(id)
            .remove::<SyncMark>()
            .insert(SyncEntity { uuid });
        debug!("New entity tracked on client {}", uuid);
    }
}

pub(crate) fn entity_parented_on_client(
    mut client: ResMut<RenetClient>,
    query: Query<(&Parent, &SyncEntity), Changed<Parent>>,
    query_parent: Query<(Entity, &SyncEntity), With<Children>>,
) {
    for (p, sup) in query.iter() {
        let Ok(parent) = query_parent.get(p.get()) else {
            continue;
        };
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntityParented {
                entity_id: sup.uuid,
                parent_id: parent.1.uuid,
            })
            .unwrap(),
        );
    }
}

pub(crate) fn entity_removed_from_client(
    mut client: ResMut<RenetClient>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, With<SyncEntity>>,
) {
    let mut despawned_entities = HashSet::new();
    track.uuid_to_entity.retain(|&s_e_id, &mut e_id| {
        if query.get(e_id).is_err() {
            despawned_entities.insert(s_e_id);
            false
        } else {
            true
        }
    });
    for &id in despawned_entities.iter() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntityDelete { id }).unwrap(),
        );
    }
}

pub(crate) fn react_on_changed_components(
    registry: Res<AppTypeRegistry>,
    mut client: ResMut<RenetClient>,
    mut track: ResMut<SyncTrackerRes>,
) {
    let registry = registry.read();
    while let Some(change) = track.changed_components_to_send.pop_front() {
        let bin = match reflect_to_bin(change.data.as_reflect(), &registry) {
            Ok(bin) => bin,
            Err(e) => {
                debug!(
                    "Could not send component {:?}, {:?}",
                    change.change_id.name, e
                );
                continue;
            }
        };
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::ComponentUpdated {
                id: change.change_id.id,
                name: change.change_id.name,
                data: bin,
            })
            .unwrap(),
        );
    }
}

pub(crate) fn react_on_changed_materials(
    mut track: ResMut<SyncTrackerRes>,
    registry: Res<AppTypeRegistry>,
    mut client: ResMut<RenetClient>,
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
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::StandardMaterialUpdated {
                        id: *id,
                        material: bin,
                    })
                    .unwrap(),
                );
            }
            AssetEvent::Removed { id: _ } => {}
            _ => (),
        }
    }
}

pub(crate) fn react_on_changed_audios(
    mut track: ResMut<SyncTrackerRes>,
    mut sync_asset: ResMut<SyncAssetTransfer>,
    mut client: ResMut<RenetClient>,
    assets: Res<Assets<AudioSource>>,
    mut events: EventReader<AssetEvent<AudioSource>>,
) {
    for event in &mut events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(asset) = assets.get(*id) else {
                    continue;
                };
                let AssetId::Uuid { uuid: id } = id else {
                    continue;
                };
                if track.skip_network_handle_change(*id) {
                    continue;
                }
                let url = sync_asset.serve_audio(id, asset);
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::AudioUpdated { id: *id, url }).unwrap(),
                );
            }
            AssetEvent::Removed { id: _ } => {}
            _ => (),
        }
    }
}

pub(crate) fn react_on_changed_meshes(
    mut track: ResMut<SyncTrackerRes>,
    mut sync_asset: ResMut<SyncAssetTransfer>,
    mut client: ResMut<RenetClient>,
    assets: Res<Assets<Mesh>>,
    mut events: EventReader<AssetEvent<Mesh>>,
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
                let url = sync_asset.serve_mesh(id, mesh);
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::MeshUpdated { id: *id, url }).unwrap(),
                );
            }
            AssetEvent::Removed { id: _ } => {}
            _ => (),
        }
    }
}

pub(crate) fn react_on_changed_images(
    mut track: ResMut<SyncTrackerRes>,
    mut sync_asset: ResMut<SyncAssetTransfer>,
    mut client: ResMut<RenetClient>,
    assets: Res<Assets<Image>>,
    mut events: EventReader<AssetEvent<Image>>,
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
                let url = sync_asset.serve_image(id, image);
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::ImageUpdated { id: *id, url }).unwrap(),
                );
            }
            AssetEvent::Removed { id: _ } => {}
            _ => (),
        }
    }
}
