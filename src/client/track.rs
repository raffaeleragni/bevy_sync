use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetClient};

use crate::{
    lib_priv::SyncTrackerRes,
    proto::Message,
    proto_serde::{compo_to_bin, mesh_to_bin},
    SyncMark, SyncUp,
};

pub(crate) fn track_spawn_client(
    mut track: ResMut<SyncTrackerRes>,
    query: Query<(Entity, &SyncUp), Added<SyncUp>>,
) {
    for (e_id, sync_up) in query.iter() {
        track
            .server_to_client_entities
            .insert(sync_up.server_entity_id, e_id);
    }
}

pub(crate) fn entity_created_on_client(
    mut client: ResMut<RenetClient>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    for id in query.iter_mut() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
        );
    }
}

pub(crate) fn entity_parented_on_client(
    mut client: ResMut<RenetClient>,
    query: Query<(&Parent, &SyncUp), Changed<Parent>>,
    query_parent: Query<(Entity, &SyncUp), With<Children>>,
) {
    for (p, sup) in query.iter() {
        let Ok(parent) = query_parent.get_component::<SyncUp>(p.get()) else {return};
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntityParented {
                server_entity_id: sup.server_entity_id,
                server_parent_id: parent.server_entity_id,
            })
            .unwrap(),
        );
    }
}

pub(crate) fn entity_removed_from_client(
    mut client: ResMut<RenetClient>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, With<SyncUp>>,
) {
    let mut despawned_entities = HashSet::new();
    track
        .server_to_client_entities
        .retain(|&s_e_id, &mut e_id| {
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
    let registry = registry.clone();
    let registry = registry.read();
    while let Some(change) = track.changed_components.pop_front() {
        debug!(
            "Component was changed on client: {}",
            change.data.type_name()
        );
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::ComponentUpdated {
                id: change.change_id.id,
                name: change.change_id.name.clone(),
                data: compo_to_bin(change.data.clone_value(), &registry),
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
    let registry = registry.clone();
    let registry = registry.read();
    for event in &mut events {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                let Some(material) = materials.get(handle) else { return; };
                if track.skip_network_handle_change(handle.id()) {
                    return;
                }
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::StandardMaterialUpdated {
                        id: handle.id(),
                        material: compo_to_bin(material.clone_value(), &registry),
                    })
                    .unwrap(),
                );
            }
            AssetEvent::Removed { handle: _ } => {}
        }
    }
}

pub(crate) fn react_on_changed_meshes(
    mut track: ResMut<SyncTrackerRes>,
    mut client: ResMut<RenetClient>,
    assets: Res<Assets<Mesh>>,
    mut events: EventReader<AssetEvent<Mesh>>,
) {
    for event in &mut events {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                let Some(mesh) = assets.get(handle) else { return; };
                if track.skip_network_handle_change(handle.id()) {
                    return;
                }
                client.send_message(
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::MeshUpdated {
                        id: handle.id(),
                        mesh: mesh_to_bin(mesh),
                    })
                    .unwrap(),
                );
            }
            AssetEvent::Removed { handle: _ } => {}
        }
    }
}
