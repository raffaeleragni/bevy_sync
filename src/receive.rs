use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::{proto::Message, SyncDown};

pub struct ReceivePlugin;

impl Plugin for ReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(query_received);
    }
}

fn query_received(
    mut commands: Commands,
    opt_client: Option<ResMut<RenetClient>>,
    opt_server: Option<ResMut<RenetServer>>,
) {
    if let Some(client) = opt_client {
        receive_as_client(client, &mut commands);
    }
    if let Some(server) = opt_server {
        receive_as_server(server, &mut commands);
    }
}

fn receive_as_client(mut client: ResMut<RenetClient>, commands: &mut Commands) {
    while let Some(message) = client.receive_message(DefaultChannel::Reliable) {
        let deser_message = bincode::deserialize(&message).unwrap();
        receive_message(MessageProcessorType::Client, deser_message, commands);
    }
}

fn receive_as_server(mut server: ResMut<RenetServer>, commands: &mut Commands) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::Reliable) {
            let deser_message = bincode::deserialize(&message).unwrap();
            receive_message(
                MessageProcessorType::Server { client_id },
                deser_message,
                commands,
            );
        }
    }
}

enum MessageProcessorType {
    Server { client_id: u64 },
    Client,
}

fn receive_message(
    connection_type: MessageProcessorType,
    server_message: Message,
    commands: &mut Commands,
) {
    match server_message {
        Message::ComponentUpdated {
            id,
            type_id: _,
            data: _,
        } => {
            commands.spawn(SyncDown { server_entity: id });
        }
        Message::ClientConnected { client_id: _ } => todo!(),
        Message::ClientDisconnected { client_id: _ } => todo!(),
        Message::ComponentRegistered {
            component_id: _,
            component_type_name: _,
        } => todo!(),
        Message::ComponentDestroyed { id: _, type_id: _ } => todo!(),
    }
}
