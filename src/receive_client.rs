use bevy::prelude::{App, Commands, Plugin, ResMut};
use bevy_renet::renet::{DefaultChannel, RenetClient};

use crate::{data::SyncTrackerRes, proto::Message, SyncMark, SyncUp};

pub(crate) struct ClientReceivePlugin;
impl Plugin for ClientReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_client);
    }
}

fn check_client(
    mut commands: Commands,
    mut track: ResMut<SyncTrackerRes>,
    opt_client: Option<ResMut<RenetClient>>,
) {
    if let Some(mut client) = opt_client {
        receive_as_client(&mut client, &mut track, &mut commands);
    }
}

fn receive_as_client(
    client: &mut ResMut<RenetClient>,
    track: &mut ResMut<SyncTrackerRes>,
    commands: &mut Commands,
) {
    while let Some(message) = client.receive_message(DefaultChannel::Reliable) {
        let deser_message = bincode::deserialize(&message).unwrap();
        client_received_a_message(deser_message, track, commands);
    }
}

fn client_received_a_message(msg: Message, track: &mut ResMut<SyncTrackerRes>, cmd: &mut Commands) {
    match msg {
        Message::EntitySpawn { id } => {
            cmd.spawn(SyncUp {
                server_entity_id: id,
            });
        }
        Message::EntitySpawnBack {
            server_entity_id: id,
            client_entity_id: back_id,
        } => {
            if let Some(mut e) = cmd.get_entity(back_id) {
                e.remove::<SyncMark>().insert(SyncUp {
                    server_entity_id: id,
                });
            }
        }
        Message::EntityDelete { id } => {
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {return};
            cmd.entity(e_id).despawn();
        }
        // No meaning on client side for these
        Message::SequenceConfirm { id: _ } => {}
        Message::SequenceRepeat { id: _ } => {}
    }
}
