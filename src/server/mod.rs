use bevy::prelude::*;
use bevy_renet::renet::{
    transport::{NetcodeClientTransport, NetcodeServerTransport},
    DefaultChannel, RenetServer, ServerEvent,
};

use crate::{
    lib_priv::{sync_audio_enabled, sync_material_enabled, sync_mesh_enabled, SyncTrackerRes},
    proto::{Message, PromoteToHostEvent},
    server::initial_sync::send_initial_sync,
    ServerState,
};

use self::track::{
    entity_created_on_server, entity_parented_on_server, entity_removed_from_server,
    react_on_changed_audios, react_on_changed_components, react_on_changed_images,
    react_on_changed_materials, react_on_changed_meshes,
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

        app.add_systems(
            Update,
            (
                entity_removed_from_server,
                entity_created_on_server,
                entity_parented_on_server,
                react_on_changed_components,
                react_on_changed_materials.run_if(sync_material_enabled),
                react_on_changed_images.run_if(sync_material_enabled),
                react_on_changed_meshes.run_if(sync_mesh_enabled),
                react_on_changed_audios.run_if(sync_audio_enabled),
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

fn client_connected(
    mut cmd: Commands,
    mut server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    mut tracker: ResMut<SyncTrackerRes>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client connected with client id: {}", client_id);
                // remove any previous pending client since the instance is a server now
                // this clients can be pending after a host promotion
                cmd.remove_resource::<NetcodeClientTransport>();
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                if tracker.host_promotion_in_progress {
                    info!(
                        "Promotion: Client flushed after host promotion with client id: {}, reason: {}",
                        client_id, reason
                    );
                } else {
                    info!(
                        "Client disconnected with client id: {}, reason: {}",
                        client_id, reason
                    );
                }

                // After all clients finished disconnecting, reset the state as
                // if promotion never happened
                if server.connected_clients() == 0 && tracker.host_promotion_in_progress {
                    info!("Promotion: Last client disconnected after a promotion to client, closing server.");
                    server.disconnect_all();
                    cmd.remove_resource::<NetcodeServerTransport>();
                    tracker.host_promotion_in_progress = false;
                }
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
