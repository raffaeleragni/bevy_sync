use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::{proto::Message, SyncEntitySpawnedFromClient};

use super::SyncDown;

pub struct ServerSendPlugin;
pub struct ClientSendPlugin;

impl Plugin for ServerSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(entity_created_on_server);
    }
}

impl Plugin for ClientSendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(entity_created_on_client);
    }
}

fn entity_created_on_server(
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<Entity, Added<SyncDown>>,
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
        }
    }
}

fn entity_created_on_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut query: Query<Entity, Added<SyncEntitySpawnedFromClient>>,
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
