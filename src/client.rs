use bevy::{
    ecs::schedule::run_enter_schedule,
    prelude::{
        resource_removed, state_exists_and_equals, Added, App, AppTypeRegistry, Commands, CoreSet,
        Entity, IntoSystemAppConfig, IntoSystemConfig, IntoSystemConfigs, NextState, OnEnter,
        OnExit, OnUpdate, Plugin, Query, Res, ResMut,
    },
    utils::HashSet,
};
use bevy_renet::renet::{transport::NetcodeClientTransport, DefaultChannel, RenetClient};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::compo_to_bin, ClientState, SyncMark,
    SyncPusher, SyncUp,
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

        app.add_system(client_reset.in_schedule(OnExit(ClientState::Connected)))
            .add_system(
                client_connect_request_initial_sync.in_schedule(OnEnter(ClientState::Connected)),
            )
            .add_systems(
                (
                    entity_created_on_client,
                    entity_removed_from_client,
                    track_spawn_client,
                    react_on_changed_components,
                )
                    .chain()
                    .in_set(OnUpdate(ClientState::Connected)),
            );
    }
}

fn client_disconnected(mut client_state: ResMut<NextState<ClientState>>) {
    client_state.set(ClientState::Disconnected);
}

fn client_connecting(mut client_state: ResMut<NextState<ClientState>>) {
    client_state.set(ClientState::Connecting);
}

fn client_connected(mut client_state: ResMut<NextState<ClientState>>) {
    client_state.set(ClientState::Connected);
}

fn client_reset(mut cmd: Commands) {
    cmd.insert_resource(SyncTrackerRes::default());
}

fn client_connect_request_initial_sync(opt_client: Option<ResMut<RenetClient>>) {
    let Some(mut client) = opt_client else { return };
    client.send_message(
        DefaultChannel::ReliableOrdered,
        bincode::serialize(&Message::InitialSync {}).unwrap(),
    );
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
    query: Query<Entity>,
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
            bincode::serialize(&Message::EntityComponentUpdated {
                id: change.id,
                name: change.name.clone(),
                data: compo_to_bin(change.data.clone_value(), &registry),
            })
            .unwrap(),
        );
    }
}
