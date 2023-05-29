use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer, ServerEvent};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::bin_to_compo, server::send_initial_sync,
    ServerState, SyncClientGeneratedEntity,
};

pub(crate) struct ServerReceivePlugin;

impl Plugin for ServerReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (client_connected, check_server)
                .chain()
                .in_set(OnUpdate(ServerState::Connected)),
        );
    }
}

fn client_connected(mut cmd: Commands, mut server_events: EventReader<ServerEvent>) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let c_id = client_id.clone();
                cmd.add(move |world: &mut World| send_initial_sync(c_id, world));
            }
            ServerEvent::ClientDisconnected {
                client_id: _,
                reason: _,
            } => {}
        }
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
            let e_id = cmd
                .spawn(SyncClientGeneratedEntity {
                    client_id,
                    client_entity_id: id,
                })
                .id();
            // Need to update the map right away or else adjacent messages won't see each other entity
            //track.server_to_client_entities.insert(e_id, e_id);
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
