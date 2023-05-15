use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::{proto::Message, SyncClientGeneratedEntity, SyncMark, SyncUp};

use super::SyncDown;

// Keeps mapping of server entity ids to client entity ids.
// Key: server entity id.
// Value: client entity id.
// For servers, the map contains same key & value.
#[derive(Resource, Default)]
pub(crate) struct SyncTrackerRes {
    pub(crate) server_to_client_entities: HashMap<Entity, Entity>,
}

pub(crate) struct ServerSendPlugin;
pub(crate) struct ClientSendPlugin;

impl Plugin for ServerSendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();
        app.add_system(track_spawn_server);
        app.add_system(entity_created_on_server);
        app.add_system(reply_back_to_client_generated_entity);
        app.add_system(entity_removed_from_server);
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
                DefaultChannel::Reliable,
                bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
            );
        }
        let mut entity = commands.entity(id);
        entity
            .remove::<SyncMark>()
            .insert(SyncDown { changed: false });
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
            DefaultChannel::Reliable,
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
                    DefaultChannel::Reliable,
                    bincode::serialize(&Message::EntitySpawn { id: entity_id }).unwrap(),
                );
            }
        }
        let mut entity = commands.entity(entity_id);
        entity
            .remove::<SyncClientGeneratedEntity>()
            .insert(SyncDown { changed: false });
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
                DefaultChannel::Reliable,
                bincode::serialize(&Message::EntityDelete { id }).unwrap(),
            );
        }
    }
}

impl Plugin for ClientSendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();
        app.add_system(track_spawn_client);
        app.add_system(entity_created_on_client);
        app.add_system(entity_removed_from_client);
    }
}

fn track_spawn_client(
    mut track: ResMut<SyncTrackerRes>,
    query: Query<(Entity, &SyncUp), Added<SyncUp>>,
) {
    for (e_id, sync_up) in query.iter() {
        track
            .server_to_client_entities
            .insert(sync_up.server_entity_id, e_id);
    }
}

fn entity_created_on_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    let Some(mut client) = opt_client else { return };
    for id in query.iter_mut() {
        client.send_message(
            DefaultChannel::Reliable,
            bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
        );
    }
}

fn entity_removed_from_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity>,
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
    let Some(mut client) = opt_client else { return };
    for &id in despawned_entities.iter() {
        client.send_message(
            DefaultChannel::Reliable,
            bincode::serialize(&Message::EntityDelete { id }).unwrap(),
        );
    }
}
