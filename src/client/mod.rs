use bevy::{ecs::schedule::run_enter_schedule, prelude::*, utils::HashSet};
use bevy_renet::renet::{transport::NetcodeClientTransport, DefaultChannel, RenetClient};

use crate::{
    lib_priv::SyncTrackerRes, proto::Message, proto_serde::compo_to_bin, ClientState, SyncMark,
    SyncUp,
};

mod receiver;

pub(crate) struct ClientSyncPlugin;
impl Plugin for ClientSyncPlugin {
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
                receiver::poll_for_messages,
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
