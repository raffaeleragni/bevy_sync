use bevy::prelude::*;
use bevy_renet::renet::{
    transport::NetcodeServerTransport, DefaultChannel, RenetServer, ServerEvent,
};

use crate::{
    lib_priv::{
        sync_material_enabled, sync_mesh_enabled, SyncClientGeneratedEntity, SyncTrackerRes,
    },
    proto::Message,
    server::initial_sync::send_initial_sync,
    ServerState,
};

use self::track::{
    entity_created_on_server, entity_parented_on_server, entity_removed_from_server,
    react_on_changed_components, react_on_changed_materials, react_on_changed_meshes,
    reply_back_to_client_generated_entity, track_spawn_server, react_on_changed_images,
};

mod initial_sync;
mod receiver;
mod track;

pub(crate) struct ServerSyncPlugin;

impl Plugin for ServerSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();

        app.add_state::<ServerState>();
        app.add_systems(
            Update,
            server_connected
                .run_if(state_exists_and_equals(ServerState::Disconnected))
                .run_if(resource_added::<NetcodeServerTransport>()),
        );
        app.add_systems(
            Update,
            server_disconnected
                .run_if(state_exists_and_equals(ServerState::Connected))
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
            )
                .chain()
                .run_if(resource_exists::<RenetServer>())
                .run_if(state_exists_and_equals(ServerState::Connected)),
        );
        app.add_systems(
            Update,
            (client_connected, receiver::poll_for_messages)
                .chain()
                .run_if(resource_exists::<RenetServer>())
                .run_if(state_exists_and_equals(ServerState::Connected)),
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
