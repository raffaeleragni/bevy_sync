use bevy::prelude::{
    App, AppTypeRegistry, Commands, Entity, Plugin, ReflectComponent, ResMut, World,
};
use bevy_renet::renet::{DefaultChannel, RenetClient};
use bincode::{DefaultOptions, Options};
use serde::de::DeserializeSeed;

use crate::{data::SyncTrackerRes, proto::Message, proto_serde::bin_to_compo, SyncMark, SyncUp};

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
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
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
        Message::EntityComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {return};
            let mut entity = cmd.entity(e_id);
            entity.add(move |_: Entity, world: &mut World| {
                let registry = world.resource::<AppTypeRegistry>().clone();
                let registry = registry.read();
                let component_data = bin_to_compo(data, &registry);
                let registration = registry.get_with_name(name.as_str()).unwrap();
                let reflect_component = registration.data::<ReflectComponent>().unwrap();
                reflect_component
                    .apply_or_insert(&mut world.entity_mut(e_id), component_data.as_reflect());
            });
        }
    }
}