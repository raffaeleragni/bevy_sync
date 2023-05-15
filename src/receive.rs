use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::{proto::Message, SyncClientGeneratedEntity, SyncMark, SyncUp};

pub struct ServerReceivePlugin;
pub struct ClientReceivePlugin;

impl Plugin for ServerReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_server);
    }
}

impl Plugin for ClientReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_client);
    }
}

fn check_server(mut commands: Commands, opt_server: Option<ResMut<RenetServer>>) {
    if let Some(mut server) = opt_server {
        receive_as_server(&mut server, &mut commands);
    }
}

fn check_client(mut commands: Commands, opt_client: Option<ResMut<RenetClient>>) {
    if let Some(mut client) = opt_client {
        receive_as_client(&mut client, &mut commands);
    }
}

fn receive_as_server(server: &mut ResMut<RenetServer>, commands: &mut Commands) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::Reliable) {
            let deser_message = bincode::deserialize(&message).unwrap();
            server_received_a_message(client_id, deser_message, commands);
        }
    }
}

fn receive_as_client(client: &mut ResMut<RenetClient>, commands: &mut Commands) {
    while let Some(message) = client.receive_message(DefaultChannel::Reliable) {
        let deser_message = bincode::deserialize(&message).unwrap();
        client_received_a_message(deser_message, commands);
    }
}

fn server_received_a_message(client_id: u64, msg: Message, cmd: &mut Commands) {
    match msg {
        Message::SequenceConfirm { id: _ } => todo!(),
        Message::SequenceRepeat { id: _ } => todo!(),
        Message::EntitySpawn { id } => {
            cmd.spawn(SyncClientGeneratedEntity {
                client_id,
                client_entity_id: id,
            });
        }
        Message::EntityDelete { id: _ } => todo!(),
        // This has no meaning on server side
        Message::EntitySpawnBack {
            server_entity_id: _,
            client_entity_id: _,
        } => {}
    }
}

fn client_received_a_message(msg: Message, cmd: &mut Commands) {
    match msg {
        Message::EntitySpawn { id } => {
            cmd.spawn(SyncUp {
                changed: false,
                server_entity_id: id,
            });
        }
        Message::EntitySpawnBack {
            server_entity_id: id,
            client_entity_id: back_id,
        } => {
            if let Some(mut e) = cmd.get_entity(back_id) {
                e.remove::<SyncMark>().insert(SyncUp {
                    changed: true,
                    server_entity_id: id,
                });
            }
        }
        Message::EntityDelete { id: _ } => todo!(),
        // No meaning on client side for these
        Message::SequenceConfirm { id: _ } => {}
        Message::SequenceRepeat { id: _ } => {}
    }
}
