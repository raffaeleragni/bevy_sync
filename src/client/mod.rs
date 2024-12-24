use bevy::prelude::*;
use bevy_renet::{netcode::NetcodeClientTransport, renet::{DefaultChannel, RenetClient}};

use crate::{
    full_sync,
    lib_priv::{sync_audio_enabled, sync_material_enabled, sync_mesh_enabled, SyncTrackerRes},
    proto::Message,
    ClientState,
};

use self::track::{
    entity_created_on_client, entity_parented_on_client, entity_removed_from_client,
    react_on_changed_audios, react_on_changed_components, react_on_changed_images,
    react_on_changed_materials, react_on_changed_meshes,
};

mod receiver;
mod track;

#[derive(Resource, Default)]
struct ClientPresendInitialSync {
    messages: Vec<Message>,
}

pub(crate) struct ClientSyncPlugin;
impl Plugin for ClientSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientPresendInitialSync>();
        app.add_systems(
            Update,
            set_client_to_connecting
                .run_if(resource_exists::<RenetClient>)
                .run_if(resource_added::<NetcodeClientTransport>)
                .run_if(in_state(ClientState::Disconnected)),
        );
        app.add_systems(
            Update,
            verify_client_connected
                .run_if(resource_exists::<RenetClient>)
                .run_if(resource_exists::<NetcodeClientTransport>)
                .run_if(in_state(ClientState::Connecting)),
        );
        app.add_systems(
            Update,
            set_client_to_disconnected
                .run_if(resource_exists::<RenetClient>)
                .run_if(resource_removed::<NetcodeClientTransport>)
                .run_if(in_state(ClientState::Connected)),
        );

        app.add_systems(
            Update,
            (
                entity_removed_from_client,
                entity_created_on_client,
                entity_parented_on_client,
                react_on_changed_components,
                react_on_changed_materials.run_if(sync_material_enabled),
                react_on_changed_images.run_if(sync_material_enabled),
                react_on_changed_meshes.run_if(sync_mesh_enabled),
                react_on_changed_audios.run_if(sync_audio_enabled),
                receiver::poll_for_messages,
            )
                .chain()
                .run_if(resource_exists::<RenetClient>)
                .run_if(resource_exists::<NetcodeClientTransport>)
                .run_if(in_state(ClientState::Connected)),
        );
    }
}

fn set_client_to_disconnected(mut client_state: ResMut<NextState<ClientState>>) {
    info!("Disconnected from server.");
    client_state.set(ClientState::Disconnected);
}

fn set_client_to_connecting(mut client_state: ResMut<NextState<ClientState>>) {
    info!("Connecting to server...");
    client_state.set(ClientState::Connecting);
}

fn verify_client_connected(
    mut cmd: Commands,
    mut client_state: ResMut<NextState<ClientState>>,
    client: ResMut<RenetClient>,
    mut tracker: ResMut<SyncTrackerRes>,
) {
    if !client.is_connected() {
        return;
    }
    info!("Connected to server.");
    client_state.set(ClientState::Connected);
    if !tracker.host_promotion_in_progress {
        cmd.queue(|world: &mut World| {
            info!("Starting new client session and requesting initial sync.");
            world.resource_mut::<ClientPresendInitialSync>().messages =
                full_sync::build_full_sync(world).unwrap_or_default();
            let mut client = world.resource_mut::<RenetClient>();
            client.send_message(
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::RequestInitialSync {}).unwrap(),
            );
        });
    } else {
        // Since no initial sync is being sent and connection completed,
        // now reset back as if promotin never happened.
        tracker.host_promotion_in_progress = false;
        info!("Promotion: Reconnected after host promotion, not requesting initial sync.");
    }
}
