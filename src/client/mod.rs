use bevy::{
    ecs::schedule::run_enter_schedule,
    prelude::{
        debug, info, resource_removed, state_exists_and_equals, Added, App, AppTypeRegistry,
        BuildWorldChildren, Changed, Children, Commands, CoreSet, Entity, IntoSystemAppConfig,
        IntoSystemConfig, IntoSystemConfigs, NextState, OnExit, OnUpdate, Parent, Plugin, Query,
        Res, ResMut, With, World,
    },
    utils::HashSet,
};
use bevy_renet::renet::{transport::NetcodeClientTransport, DefaultChannel, RenetClient};

use crate::{
    lib_priv::SyncTrackerRes, proto::Message, proto_serde::compo_to_bin, ClientState, SyncMark,
    SyncUp,
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
                entity_parented_on_client,
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

fn entity_parented_on_client(
    opt_client: Option<ResMut<RenetClient>>,
    query: Query<(&Parent, &SyncUp), Changed<Parent>>,
    query_parent: Query<(Entity, &SyncUp), With<Children>>,
) {
    let Some(mut client) = opt_client else { return };
    for (p, sup) in query.iter() {
        let Ok(parent) = query_parent.get_component::<SyncUp>(p.get()) else {return};
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntityParented {
                server_entity_id: sup.server_entity_id,
                server_parent_id: parent.server_entity_id,
            })
            .unwrap(),
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
    mut track: ResMut<SyncTrackerRes>,
) {
    let Some(mut client) = opt_client else { return; };
    let registry = registry.clone();
    let registry = registry.read();
    while let Some(change) = track.changed_components.pop_front() {
        debug!(
            "Component was changed on client: {}",
            change.data.type_name()
        );
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::ComponentUpdated {
                id: change.change_id.id,
                name: change.change_id.name.clone(),
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
        Message::EntityParented {
            server_entity_id: e_id,
            server_parent_id: p_id,
        } => {
            let Some(&c_e_id) = track.server_to_client_entities.get(&e_id) else {return};
            let Some(&c_p_id) = track.server_to_client_entities.get(&p_id) else {return};
            cmd.add(move |world: &mut World| {
                let mut entity = world.entity_mut(c_e_id);
                let opt_parent = entity.get::<Parent>();
                if opt_parent.is_none() || opt_parent.unwrap().get() != c_p_id {
                    entity.set_parent(p_id);
                    world.entity_mut(c_p_id).add_child(c_e_id);
                }
            });
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
                SyncTrackerRes::apply_component_change_from_network(e_id, name, data, world);
            });
        }
        Message::StandardMaterialUpdated { id, material } => cmd.add(move |world: &mut World| {
            SyncTrackerRes::apply_material_change_from_network(id, &material, world);
        }),
    }
}