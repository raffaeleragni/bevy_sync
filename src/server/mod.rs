use bevy::prelude::*;
use bevy_renet::renet::{
    transport::{NetcodeClientTransport, NetcodeServerTransport},
    DefaultChannel, RenetServer, ServerEvent,
};

use crate::{
    lib_priv::{
        sync_material_enabled, sync_mesh_enabled, SyncClientGeneratedEntity, SyncTrackerRes,
    },
    proto::{Message, PromoteToHostEvent},
    server::initial_sync::send_initial_sync,
    ServerState,
};

use self::track::{
    entity_created_on_server, entity_parented_on_server, entity_removed_from_server,
    react_on_changed_components, react_on_changed_images, react_on_changed_materials,
    react_on_changed_meshes, reply_back_to_client_generated_entity, track_spawn_server,
};

mod initial_sync;
mod receiver;
mod track;

pub(crate) struct ServerSyncPlugin;

impl Plugin for ServerSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            server_connected
                .run_if(resource_exists::<RenetServer>)
                .run_if(in_state(ServerState::Disconnected))
                .run_if(resource_added::<NetcodeServerTransport>),
        );
        app.add_systems(
            Update,
            server_disconnected
                .run_if(resource_exists::<RenetServer>)
                .run_if(in_state(ServerState::Connected))
                .run_if(resource_removed::<NetcodeServerTransport>()),
        );

        app.add_systems(OnExit(ServerState::Connected), server_reset);
        app.add_systems(
            Update,
            (
                reply_back_to_client_generated_entity,
                entity_created_on_server,
                entity_parented_on_server,
                entity_removed_from_server,
                track_spawn_server,
                react_on_changed_components,
                react_on_changed_materials.run_if(sync_material_enabled),
                react_on_changed_images.run_if(sync_material_enabled),
                react_on_changed_meshes.run_if(sync_mesh_enabled),
                promote_to_host_event_reader,
            )
                .chain()
                .run_if(resource_exists::<RenetServer>)
                .run_if(resource_exists::<NetcodeServerTransport>)
                .run_if(in_state(ServerState::Connected)),
        );
        app.add_systems(
            Update,
            (client_connected, receiver::poll_for_messages)
                .chain()
                .run_if(resource_exists::<RenetServer>)
                .run_if(resource_exists::<NetcodeServerTransport>)
                .run_if(in_state(ServerState::Connected)),
        );
    }
}

fn client_connected(mut cmd: Commands, mut server_events: EventReader<ServerEvent>) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client connected with client id: {}", client_id);
                cmd.add(move |world: &mut World| send_initial_sync(client_id, world));
                // remove any previous pending client since the instance is a server now
                // this clients can be pending after a host promotion
                cmd.remove_resource::<NetcodeClientTransport>();
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!(
                    "Client disconnected with client id: {}, reason: {}",
                    client_id, reason
                );
            }
        }
    }
}

fn server_disconnected(mut state: ResMut<NextState<ServerState>>) {
    info!("Server is shut down.");
    state.set(ServerState::Disconnected);
}

fn server_connected(mut state: ResMut<NextState<ServerState>>) {
    info!("Server ready to accept connections.");
    state.set(ServerState::Connected);
}

fn server_reset(mut cmd: Commands) {
    cmd.insert_resource(SyncTrackerRes::default());
}

fn promote_to_host_event_reader(
    mut server: ResMut<RenetServer>,
    mut events: EventReader<PromoteToHostEvent>,
) {
    for event in events.read() {
        info!("Promoting {} to host", event.id);
        server.send_message(
            event.id,
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::PromoteToHost {}).unwrap(),
        );
    }
}
