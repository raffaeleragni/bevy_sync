use bevy::{
    ecs::system::Command,
    prelude::{
        App, AppTypeRegistry, Commands, Entity, Plugin, ReflectComponent, Res, ResMut, World,
    },
    reflect::{Reflect, TypeRegistryInternal},
};
use bevy_renet::renet::{DefaultChannel, RenetClient};

use crate::{data::SyncTrackerRes, proto::Message, SyncMark, SyncUp};

pub(crate) struct ClientReceivePlugin;
impl Plugin for ClientReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_client);
    }
}

fn check_client(
    registry: Res<AppTypeRegistry>,
    mut commands: Commands,
    mut track: ResMut<SyncTrackerRes>,
    opt_client: Option<ResMut<RenetClient>>,
) {
    if let Some(mut client) = opt_client {
        let registry = registry.clone();
        let registry = registry.read();
        receive_as_client(&registry, &mut client, &mut track, &mut commands);
    }
}

fn receive_as_client(
    registry: &TypeRegistryInternal,
    client: &mut ResMut<RenetClient>,
    track: &mut ResMut<SyncTrackerRes>,
    commands: &mut Commands,
) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        let deser_message = bincode::deserialize(&message).unwrap();
        client_received_a_message(registry, deser_message, track, commands);
    }
}

fn client_received_a_message(
    registry: &TypeRegistryInternal,
    msg: Message,
    track: &mut ResMut<SyncTrackerRes>,
    cmd: &mut Commands,
) {
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
            entity.add(|e: Entity, world: &mut World| {
                let registry = world.resource::<AppTypeRegistry>().clone();
                let registry = registry.read();
                let registration = registry.get_with_name(name.as_str()).unwrap();
                let reflect_component = registration.data::<ReflectComponent>().unwrap();
                reflect_component.apply_or_insert(&mut world.entity_mut(e_id), data.as_reflect());
            });
        }
    }
}
