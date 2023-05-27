use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{proto::Message, SyncClientGeneratedEntity};

pub(crate) struct ServerReceivePlugin;

impl Plugin for ServerReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_server);
    }
}

fn check_server(mut commands: Commands, opt_server: Option<ResMut<RenetServer>>) {
    if let Some(mut server) = opt_server {
        receive_as_server(&mut server, &mut commands);
    }
}

fn receive_as_server(server: &mut ResMut<RenetServer>, commands: &mut Commands) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let deser_message = bincode::deserialize(&message).unwrap();
            server_received_a_message(client_id, deser_message, commands);
        }
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
        Message::EntityDelete { id } => {
            if let Some(mut e) = cmd.get_entity(id) {
                e.despawn();
            }
        }
        // This has no meaning on server side
        Message::EntitySpawnBack {
            server_entity_id: _,
            client_entity_id: _,
        } => {}
        Message::EntityComponentUpdated {
            id: _,
            name: _,
            data: _,
        } => {}
    }
}
