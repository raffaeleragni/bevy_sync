use bevy::prelude::*;
use bevy_renet::renet::{
    transport::{NetcodeClientTransport, NetcodeServerTransport},
    DefaultChannel, RenetClient,
};

use crate::{
    lib_priv::{
        sync_audio_enabled, sync_material_enabled, sync_mesh_enabled, PromotionState,
        SyncTrackerRes,
    },
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

pub(crate) struct ClientSyncPlugin;
impl Plugin for ClientSyncPlugin {
    fn build(&self, app: &mut App) {
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
                .run_if(resource_removed::<NetcodeClientTransport>())
                .run_if(in_state(ClientState::Connected)),
        );

        app.add_systems(
            OnExit(ClientState::Connected),
            client_reset.run_if(in_state(PromotionState::NeverPromoted)),
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
    mut client_state: ResMut<NextState<ClientState>>,
    mut cmd: Commands,
    promotion_state: Res<State<PromotionState>>,
    mut client: ResMut<RenetClient>,
) {
    if !client.is_connected() {
        return;
    }
    info!("Connected to server.");
    client_state.set(ClientState::Connected);
    if promotion_state.eq(&PromotionState::NeverPromoted) {
        debug!("Starting new client session");
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::RequestInitialSync {}).unwrap(),
        );
    }
    // remove any previous pending server since the instance is a client now
    // this servers can be pending after a host promotion
    cmd.remove_resource::<NetcodeServerTransport>();
}

fn client_reset(mut cmd: Commands) {
    cmd.insert_resource(SyncTrackerRes::default());
}
