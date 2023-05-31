use bevy::{ecs::schedule::run_enter_schedule, prelude::*, utils::HashSet};
use bevy_renet::renet::{transport::NetcodeClientTransport, DefaultChannel, RenetClient};

use crate::{
    data::SyncTrackerRes,
    proto::Message,
    proto_serde::{bin_to_compo, compo_to_bin},
    ClientState, SyncMark, SyncPusher, SyncUp,
};

pub(crate) struct ClientSendPlugin;
impl Plugin for ClientSendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();

        app.add_state::<ClientState>();
        app.add_systems(
            (
                client_disconnected.run_if(resource_removed::<NetcodeClientTransport>()),
                client_connecting
                    .run_if(bevy_renet::transport::client_connecting)
                    .run_if(state_exists_and_equals(ClientState::Disconnected)),
                client_connected
                    .run_if(bevy_renet::transport::client_connected)
                    .run_if(state_exists_and_equals(ClientState::Connecting)),
            )
                .before(run_enter_schedule::<ClientState>)
                .in_base_set(CoreSet::StateTransitions),
        );

        app.add_system(client_reset.in_schedule(OnExit(ClientState::Connected)));
        app.add_systems(
            (
                track_spawn_client,
                entity_created_on_client,
                react_on_changed_components,
                entity_removed_from_client,
                poll_for_messages,
            )
                .chain()
                .in_set(OnUpdate(ClientState::Connected)),
        );
    }
}

fn client_disconnected(mut client_state: ResMut<NextState<ClientState>>) {
    info!("Disconnected from server.");
    client_state.set(ClientState::Disconnected);
}

fn client_connecting(mut client_state: ResMut<NextState<ClientState>>) {
    info!("Connecting to server...");
    client_state.set(ClientState::Connecting);
}

fn client_connected(mut client_state: ResMut<NextState<ClientState>>) {
    info!("Connected to server.");
    client_state.set(ClientState::Connected);
}

fn client_reset(mut cmd: Commands) {
    cmd.insert_resource(SyncTrackerRes::default());
}

fn track_spawn_client(
    mut track: ResMut<SyncTrackerRes>,
    query: Query<(Entity, &SyncUp), Added<SyncUp>>,
) {
    for (e_id, sync_up) in query.iter() {
        track
            .server_to_client_entities
            .insert(sync_up.server_entity_id, e_id);
    }
}

fn entity_created_on_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    let Some(mut client) = opt_client else { return };
    for id in query.iter_mut() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
        );
    }
}

fn entity_removed_from_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, With<SyncUp>>,
) {
    let mut despawned_entities = HashSet::new();
    track
        .server_to_client_entities
        .retain(|&s_e_id, &mut e_id| {
            if query.get(e_id).is_err() {
                despawned_entities.insert(s_e_id);
                false
            } else {
                true
            }
        });
    let Some(mut client) = opt_client else { return };
    for &id in despawned_entities.iter() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntityDelete { id }).unwrap(),
        );
    }
}

fn react_on_changed_components(
    registry: Res<AppTypeRegistry>,
    opt_client: Option<ResMut<RenetClient>>,
    mut track: ResMut<SyncPusher>,
) {
    let Some(mut client) = opt_client else { return; };
    let registry = registry.clone();
    let registry = registry.read();
    while let Some(change) = track.components.pop_front() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::ComponentUpdated {
                id: change.id,
                name: change.name.clone(),
                data: compo_to_bin(change.data.clone_value(), &registry),
            })
            .unwrap(),
        );
    }
}

fn poll_for_messages(
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
            debug!(
                "Client received of type EntitySpawn for server entity {}v{}",
                id.index(),
                id.generation()
            );
            if let Some(e_id) = track.server_to_client_entities.get(&id) {
                if cmd.get_entity(*e_id).is_some() {
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
            debug!(
                "Client received of type EntitySpawnBack for server entity {}v{}",
                id.index(),
                id.generation()
            );
            if let Some(mut e) = cmd.get_entity(back_id) {
                e.remove::<SyncMark>().insert(SyncUp {
                    server_entity_id: id,
                });
            }
        }
        Message::EntityDelete { id } => {
            debug!(
                "Client received of type EntityDelete for server entity {}v{}",
                id.index(),
                id.generation()
            );
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {return};
            let Some(mut e) = cmd.get_entity(e_id) else {return};
            e.despawn();
        }
        Message::ComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {return};
            let mut entity = cmd.entity(e_id);
            entity.add(move |_: Entity, world: &mut World| {
                let registry = world.resource::<AppTypeRegistry>().clone();
                let registry = registry.read();
                let component_data = bin_to_compo(&data, &registry);
                let registration = registry.get_with_name(name.as_str()).unwrap();
                let reflect_component = registration.data::<ReflectComponent>().unwrap();
                let previous_value = reflect_component.reflect(world.entity(e_id));
                if needs_to_change(previous_value, &component_data) {
                    debug!(
                        "Client received message of type ComponentUpdated for entity {}v{} and component {}",
                        id.index(),
                        id.generation(),
                        name
                    );
                    reflect_component
                        .apply_or_insert(&mut world.entity_mut(e_id), component_data.as_reflect());
                } else {
                    debug!(
                        "Skipping client received message of type ComponentUpdated for entity {}v{} and component {}",
                        id.index(),
                        id.generation(),
                        name
                    );
                }
            });
        }
    }
}

fn needs_to_change(
    previous_value: Option<&dyn Reflect>,
    component_data: &Box<dyn Reflect>,
) -> bool {
    if previous_value.is_none() {
        return true;
    }
    !previous_value
        .unwrap()
        .reflect_partial_eq(&**component_data)
        .unwrap_or_else(|| true)
}
