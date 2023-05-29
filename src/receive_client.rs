use bevy::prelude::{
    App, AppTypeRegistry, Commands, Entity, IntoSystemConfig, OnUpdate, Plugin, ReflectComponent,
    ResMut, World,
};
use bevy_renet::renet::{DefaultChannel, RenetClient};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::bin_to_compo, ClientState, SyncMark, SyncUp,
};

pub(crate) struct ClientReceivePlugin;
impl Plugin for ClientReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_client.in_set(OnUpdate(ClientState::Connected)));
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
            if let Some(e_id) = track.server_to_client_entities.get(&id) {
                if let Some(_) = cmd.get_entity(*e_id) {
                    return;
                }
            }
            let e_id = cmd
                .spawn(SyncUp {
                    server_entity_id: id,
                })
                .id();
            // Need to update the map right away or else adjacent messages won't see each other entity
            track.server_to_client_entities.insert(id, e_id);
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
            let Some(mut e) = cmd.get_entity(e_id) else {return};
            e.despawn();
        }
        Message::EntityComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {return};
            let mut entity = cmd.entity(e_id);
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
