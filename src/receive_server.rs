use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::bin_to_compo, SyncClientGeneratedEntity,
};

pub(crate) struct ServerReceivePlugin;

impl Plugin for ServerReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_server);
    }
}

fn check_server(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut track: ResMut<SyncTrackerRes>,
) {
    if let Some(mut server) = opt_server {
        receive_as_server(&mut server, &mut track, &mut commands);
    }
}

fn receive_as_server(
    server: &mut ResMut<RenetServer>,
    track: &mut ResMut<SyncTrackerRes>,
    commands: &mut Commands,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let deser_message = bincode::deserialize(&message).unwrap();
            server_received_a_message(client_id, deser_message, track, commands);
        }
    }
}

fn server_received_a_message(
    client_id: u64,
    msg: Message,
    track: &mut ResMut<SyncTrackerRes>,
    cmd: &mut Commands,
) {
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
