use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::bin_to_compo, server::build_initial_sync,
    SyncClientGeneratedEntity,
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
            server_received_a_message(client_id, deser_message, server, track, commands);
        }
    }
}

fn server_received_a_message(
    client_id: u64,
    msg: Message,
    server: &mut ResMut<RenetServer>,
    track: &mut ResMut<SyncTrackerRes>,
    cmd: &mut Commands,
) {
    match msg {
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
            repeat_except_for_client(
                client_id,
                server,
                &Message::EntityComponentUpdated {
                    id,
                    name: name.clone(),
                    data: data.clone(),
                },
            );
            entity.add(move |_: Entity, world: &mut World| {
                let registry = world.resource::<AppTypeRegistry>().clone();
                let registry = registry.read();
                let component_data = bin_to_compo(&data, &registry);
                let registration = registry.get_with_name(name.as_str()).unwrap();
                let reflect_component = registration.data::<ReflectComponent>().unwrap();
                reflect_component
                    .apply_or_insert(&mut world.entity_mut(e_id), component_data.as_reflect());
            });
        }
        /*
          Creating an initial sync means scanning the world in a blocking way.
          This is an issue that will need to be addressed somehow since users will not
          receive any server updates in the meanwhile, and the server will be frozen as
          this loops through all items.
          cmd.add 'should' queue it into another pipeline, but the loops will still be blocking and world-accessing
        */
        Message::InitialSync {} => {
            cmd.add(move |world: &mut World| {
                let mut initial_sync = build_initial_sync(world);
                let mut server = world.resource_mut::<RenetServer>();
                for msg in initial_sync.drain(..) {
                    let msg_bin = bincode::serialize(&msg).unwrap();
                    server.send_message(client_id, DefaultChannel::ReliableOrdered, msg_bin);
                }
            });
        }
    }
}

fn repeat_except_for_client(msg_client_id: u64, server: &mut ResMut<RenetServer>, msg: &Message) {
    for client_id in server.clients_id().into_iter() {
        if client_id == msg_client_id {
            continue;
        }
        server.send_message(
            client_id,
            DefaultChannel::ReliableOrdered,
            bincode::serialize(msg).unwrap(),
        );
    }
}
