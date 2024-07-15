use crate::{
    logging::{log_message_received, Who},
    networking::{assets::SyncAssetTransfer, create_client, create_server},
    proto::SyncAssetType,
    SyncConnectionParameters, SyncEntity,
};

use super::*;

pub(crate) fn poll_for_messages(
    mut commands: Commands,
    connection_parameters: Res<SyncConnectionParameters>,
    mut track: ResMut<SyncTrackerRes>,
    mut sync_assets: ResMut<SyncAssetTransfer>,
    mut client: ResMut<RenetClient>,
) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        let deser_message = bincode::deserialize(&message).unwrap();
        client_received_a_message(
            deser_message,
            &connection_parameters,
            &mut client,
            &mut track,
            &mut sync_assets,
            &mut commands,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn client_received_a_message(
    msg: Message,
    connection_parameters: &Res<SyncConnectionParameters>,
    client: &mut ResMut<RenetClient>,
    track: &mut ResMut<SyncTrackerRes>,
    sync_assets: &mut ResMut<SyncAssetTransfer>,
    cmd: &mut Commands,
) {
    log_message_received(Who::Client, &msg);
    match msg {
        Message::EntitySpawn { id } => {
            if let Some(e_id) = track.uuid_to_entity.get(&id) {
                if cmd.get_entity(*e_id).is_some() {
                    return;
                }
            }
            let e_id = cmd.spawn(SyncEntity { uuid: id }).id();
            // Need to update the map right away or else adjacent messages won't see each other entity
            track.uuid_to_entity.insert(id, e_id);
            track.entity_to_uuid.insert(e_id, id);
        }
        Message::EntityParented {
            entity_id: e_id,
            parent_id: p_id,
        } => {
            let Some(&c_e_id) = track.uuid_to_entity.get(&e_id) else {
                return;
            };
            let Some(&c_p_id) = track.uuid_to_entity.get(&p_id) else {
                return;
            };
            cmd.add(move |world: &mut World| {
                let mut entity = world.entity_mut(c_e_id);
                let opt_parent = entity.get::<Parent>();
                if opt_parent.is_none() || opt_parent.unwrap().get() != c_p_id {
                    entity.set_parent(c_p_id);
                    world.entity_mut(c_p_id).add_child(c_e_id);
                }
            });
        }
        Message::EntityDelete { id } => {
            let Some(&e_id) = track.uuid_to_entity.get(&id) else {
                return;
            };
            let Some(mut e) = cmd.get_entity(e_id) else {
                return;
            };
            track.uuid_to_entity.remove(&id);
            track.entity_to_uuid.remove(&e_id);
            e.despawn();
        }
        Message::ComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.uuid_to_entity.get(&id) else {
                return;
            };
            cmd.add(move |world: &mut World| {
                SyncTrackerRes::apply_component_change_from_network(world, e_id, name, &data);
            });
        }
        Message::StandardMaterialUpdated { id, material } => cmd.add(move |world: &mut World| {
            SyncTrackerRes::apply_material_change_from_network(id, &material, world);
        }),
        Message::MeshUpdated { id, url } => sync_assets.request(SyncAssetType::Mesh, id, url),
        Message::ImageUpdated { id, url } => sync_assets.request(SyncAssetType::Image, id, url),
        Message::AudioUpdated { id, url } => sync_assets.request(SyncAssetType::Audio, id, url),
        Message::PromoteToHost => {
            info!("Promotion: Client is being promoted to host");
            let ip = connection_parameters.ip;
            let port = connection_parameters.port;
            cmd.add(move |world: &mut World| {
                info!("Promotion: Starting as host...");
                world.insert_resource(create_server(ip, port));
                world
                    .resource_mut::<SyncTrackerRes>()
                    .host_promotion_in_progress = true;
            });
        }
        Message::NewHost {
            ip,
            port,
            web_port: _,
            max_transfer: _,
        } => {
            info!("Promotion: A new host has been promoted. Reconnecting to new host");
            client.disconnect();
            cmd.remove_resource::<NetcodeClientTransport>();
            cmd.insert_resource(create_client(ip, port));
            // even if it was a client before, this connection is not a new session
            // and won't need the initial_sync, so it's consider a client to client promotion
            track.host_promotion_in_progress = true;
        }
        // Nothing to do, only servers send initial sync
        Message::RequestInitialSync => {}
    }
}
