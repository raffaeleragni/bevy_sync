use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::compo_to_bin, SyncClientGeneratedEntity,
    SyncMark, SyncPusher,
};

use super::SyncDown;

pub(crate) struct ServerSendPlugin;

impl Plugin for ServerSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(track_spawn_server);
        app.add_system(entity_created_on_server);
        app.add_system(reply_back_to_client_generated_entity);
        app.add_system(entity_removed_from_server);
        app.add_system(react_on_changed_components);
    }
}

fn track_spawn_server(mut track: ResMut<SyncTrackerRes>, query: Query<Entity, Added<SyncDown>>) {
    for e_id in query.iter() {
        track.server_to_client_entities.insert(e_id, e_id);
    }
}

fn entity_created_on_server(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    let Some(mut server) = opt_server else { return };
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

fn reply_back_to_client_generated_entity(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<(Entity, &SyncClientGeneratedEntity), Added<SyncClientGeneratedEntity>>,
) {
    let Some(mut server) = opt_server else { return };
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

fn entity_removed_from_server(
    opt_server: Option<ResMut<RenetServer>>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity>,
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
    let Some(mut server) = opt_server else { return };
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

fn react_on_changed_components(
    registry: Res<AppTypeRegistry>,
    opt_server: Option<ResMut<RenetServer>>,
    mut track: ResMut<SyncPusher>,
) {
    let Some(mut server) = opt_server else { return; };
    let registry = registry.clone();
    let registry = registry.read();
    while let Some(change) = track.components.pop_front() {
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityComponentUpdated {
                    id: change.id,
                    name: change.name.clone(),
                    data: compo_to_bin(change.data.clone_value(), &registry),
                })
                .unwrap(),
            );
        }
    }
}