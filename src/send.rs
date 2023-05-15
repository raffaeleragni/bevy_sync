use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::{proto::Message, SyncClientGeneratedEntity, SyncMark};

use super::SyncDown;

pub struct ServerSendPlugin;

impl Plugin for ServerSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(entity_created_on_server);
        app.add_system(reply_back_to_client_generated_entity);
    }
}

fn entity_created_on_server(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    if let Some(mut server) = opt_server {
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
}

fn reply_back_to_client_generated_entity(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<(Entity, &SyncClientGeneratedEntity), Added<SyncClientGeneratedEntity>>,
) {
    if let Some(mut server) = opt_server {
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
}

pub struct ClientSendPlugin;

impl Plugin for ClientSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(entity_created_on_client);
    }
}

fn entity_created_on_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    if let Some(mut client) = opt_client {
        for id in query.iter_mut() {
            client.send_message(
                DefaultChannel::Reliable,
                bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
            );
        }
    }
}
