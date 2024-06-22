use crate::{
    lib_priv::PromotedToServer,
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
    mut send_promoted_event: EventWriter<PromotedToServer>,
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
            &mut send_promoted_event,
        );
    }
}

fn client_received_a_message(
    msg: Message,
    connection_parameters: &Res<SyncConnectionParameters>,
    client: &mut ResMut<RenetClient>,
    track: &mut ResMut<SyncTrackerRes>,
    sync_assets: &mut ResMut<SyncAssetTransfer>,
    cmd: &mut Commands,
    send_promoted_event: &mut EventWriter<PromotedToServer>,
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
                SyncTrackerRes::apply_component_change_from_network(e_id, name, &data, world);
            });
        }
        Message::StandardMaterialUpdated { id, material } => cmd.add(move |world: &mut World| {
            SyncTrackerRes::apply_material_change_from_network(id, &material, world);
        }),
        Message::MeshUpdated { id, url } => sync_assets.request(SyncAssetType::Mesh, id, url),
        Message::ImageUpdated { id, url } => sync_assets.request(SyncAssetType::Image, id, url),
        Message::PromoteToHost => {
            info!("Client is being promoted to host");
            client.send_message(
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::NewHost {
                    ip: connection_parameters.ip,
                    port: connection_parameters.port,
                    web_port: connection_parameters.web_port,
                    max_transfer: connection_parameters.max_transfer,
                })
                .unwrap(),
            );
            let ip = connection_parameters.ip;
            let port = connection_parameters.port;
            cmd.add(move |world: &mut World| {
                // cannot remove client because the message above is still queued.
                // found no way to flush client so far, but at least the previous server
                // will still disconnect all the clients, so this client should still
                // eventually disconnect
                //world.resource_mut::<RenetClient>().disconnect();
                //world.remove_resource::<NetcodeClientTransport>();
                info!("Starting as host...");
                world.insert_resource(create_server(ip, port));
            });
            send_promoted_event.send(PromotedToServer {});
        }
        Message::NewHost {
            ip,
            port,
            web_port: _,
            max_transfer: _,
        } => {
            info!("A new host has been promoted. Reconnecting to new host");
            client.disconnect();
            cmd.remove_resource::<NetcodeClientTransport>();
            cmd.insert_resource(create_client(ip, port));
        }
    }
}
