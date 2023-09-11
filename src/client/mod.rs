use bevy::prelude::*;
use bevy_renet::renet::{transport::NetcodeClientTransport, DefaultChannel, RenetClient};

use crate::{
    lib_priv::{sync_material_enabled, sync_mesh_enabled, SyncTrackerRes},
    proto::Message,
    ClientState, SyncMark, SyncUp,
};

use self::track::{
    entity_created_on_client, entity_parented_on_client, entity_removed_from_client,
    react_on_changed_components, react_on_changed_materials, react_on_changed_meshes,
    track_spawn_client,
};

mod receiver;
mod track;

pub(crate) struct ClientSyncPlugin;
impl Plugin for ClientSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();

        app.add_state::<ClientState>();

        app.add_systems(
            Update,
            client_connected
                .run_if(state_exists_and_equals(ClientState::Connecting))
                .run_if(bevy_renet::transport::client_connected()),
        );
        app.add_systems(
            Update,
            client_connecting
                .run_if(state_exists_and_equals(ClientState::Disconnected))
                .run_if(bevy_renet::transport::client_connecting()),
        );
        app.add_systems(
            Update,
            client_disconnected
                .run_if(state_exists_and_equals(ClientState::Disconnected))
                .run_if(resource_removed::<NetcodeClientTransport>()),
        );

        app.add_systems(OnExit(ClientState::Connected), client_reset);
        app.add_systems(
            Update,
            (
                track_spawn_client,
                entity_created_on_client,
                entity_parented_on_client,
                react_on_changed_components,
                react_on_changed_materials.run_if(sync_material_enabled),
                react_on_changed_meshes.run_if(sync_mesh_enabled),
                entity_removed_from_client,
                receiver::poll_for_messages,
            )
                .chain()
                .run_if(resource_exists::<RenetClient>())
                .run_if(state_exists_and_equals(ClientState::Connected)),
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
