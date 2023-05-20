use bevy::{ecs::component::ComponentId, prelude::*, utils::HashSet};
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::{data::SyncTrackerRes, proto::Message, SyncClientGeneratedEntity, SyncMark, SyncUp};

use super::SyncDown;

pub(crate) struct ServerSendPlugin;

impl Plugin for ServerSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(track_spawn_server);
        app.add_system(entity_created_on_server);
        app.add_system(reply_back_to_client_generated_entity);
        app.add_system(entity_removed_from_server);
        //TODO: component sync first case: app.add_system(track_components_server);
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
                DefaultChannel::Reliable,
                bincode::serialize(&Message::EntityDelete { id }).unwrap(),
            );
        }
    }
}

fn track_components_server(track: ResMut<SyncTrackerRes>, world: &World) {
    let Some(marker) = world.component_id::<SyncDown>() else {return;};
    for archetype in world.archetypes().iter().filter(|a| a.contains(marker)) {
        for c_id in archetype
            .components()
            .filter(|c_id| track.sync_components.contains(c_id))
        {
            for archetype_entity in archetype.entities() {
                let e_id = world.entity(archetype_entity.entity()).id();
                check_if_component_changed_on_server(e_id, c_id);
            }
        }
    }
}

fn check_if_component_changed_on_server(e_id: Entity, c_id: ComponentId) {}
