use bevy_renet::renet::ClientId;

use crate::{
    lib_priv::{PromotedToClient, PromotionState},
    logging::{log_message_received, Who},
    networking::{assets::SyncAssetTransfer, create_client},
    proto::SyncAssetType,
    SyncEntity,
};

use super::*;

pub(crate) fn poll_for_messages(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut track: ResMut<SyncTrackerRes>,
    mut sync_assets: ResMut<SyncAssetTransfer>,
    mut send_promoted_event: EventWriter<PromotedToClient>,
    mut next_promotion_state: ResMut<NextState<PromotionState>>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let deser_message = bincode::deserialize(&message).unwrap();
            server_received_a_message(
                client_id,
                deser_message,
                &mut server,
                &mut track,
                &mut sync_assets,
                &mut commands,
                &mut send_promoted_event,
                &mut next_promotion_state,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn server_received_a_message(
    client_id: ClientId,
    msg: Message,
    server: &mut ResMut<RenetServer>,
    track: &mut ResMut<SyncTrackerRes>,
    sync_assets: &mut ResMut<SyncAssetTransfer>,
    cmd: &mut Commands,
    send_promoted_event: &mut EventWriter<PromotedToClient>,
    next_promotion_state: &mut ResMut<NextState<PromotionState>>,
) {
    log_message_received(Who::Server, &msg);
    match msg {
        Message::EntitySpawn { id } => {
            let e_id = cmd.spawn(SyncEntity { uuid: id }).id();
            // Need to update the map right away or else adjacent messages won't see each other entity
            track.uuid_to_entity.insert(id, e_id);
            track.entity_to_uuid.insert(e_id, id);
            repeat_except_for_client(client_id, server, &Message::EntitySpawn { id });
        }
        Message::EntityParented {
            entity_id: me_id,
            parent_id: mp_id,
        } => {
            cmd.add(move |world: &mut World| {
                let track = world.resource::<SyncTrackerRes>();
                let Some(e_id) = track.uuid_to_entity.get(&me_id) else {
                    return;
                };
                let Some(p_id) = track.uuid_to_entity.get(&mp_id) else {
                    return;
                };
                let e_id = *e_id;
                let p_id = *p_id;
                let Some(mut entity) = world.get_entity_mut(e_id) else {
                    return;
                };
                let opt_parent = entity.get::<Parent>();
                if opt_parent.is_none() || opt_parent.unwrap().get() != p_id {
                    entity.set_parent(p_id);
                    world.entity_mut(p_id).add_child(e_id);
                }
                repeat_except_for_client(
                    client_id,
                    &mut world.resource_mut::<RenetServer>(),
                    &Message::EntityParented {
                        entity_id: me_id,
                        parent_id: mp_id,
                    },
                );
            });
        }
        Message::EntityDelete { id: mid } => {
            if let Some(id) = track.uuid_to_entity.get(&mid) {
                let id = *id;
                if let Some(mut e) = cmd.get_entity(id) {
                    e.despawn();
                    track.uuid_to_entity.remove(&mid);
                    track.entity_to_uuid.remove(&id);
                }
            }
            repeat_except_for_client(client_id, server, &Message::EntityDelete { id: mid });
        }
        Message::ComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.uuid_to_entity.get(&id) else {
                return;
            };
            cmd.add(move |world: &mut World| {
                let changed = SyncTrackerRes::apply_component_change_from_network(
                    world,
                    e_id,
                    name.clone(),
                    &data,
                );

                if changed {
                    repeat_except_for_client(
                        client_id,
                        &mut world.resource_mut::<RenetServer>(),
                        &Message::ComponentUpdated { id, name, data },
                    );
                }
            });
        }
        Message::StandardMaterialUpdated { id, material } => cmd.add(move |world: &mut World| {
            SyncTrackerRes::apply_material_change_from_network(id, &material, world);

            repeat_except_for_client(
                client_id,
                &mut world.resource_mut::<RenetServer>(),
                &Message::StandardMaterialUpdated { id, material },
            );
        }),
        Message::MeshUpdated { id, url } => {
            sync_assets.request(SyncAssetType::Mesh, id, url.clone());
            cmd.add(move |world: &mut World| {
                repeat_except_for_client(
                    client_id,
                    &mut world.resource_mut::<RenetServer>(),
                    &Message::MeshUpdated { id, url },
                );
            })
        }
        Message::ImageUpdated { id, url } => {
            sync_assets.request(SyncAssetType::Image, id, url.clone());
            cmd.add(move |world: &mut World| {
                repeat_except_for_client(
                    client_id,
                    &mut world.resource_mut::<RenetServer>(),
                    &Message::ImageUpdated { id, url },
                );
            })
        }
        // server is already host, no operation to do
        Message::PromoteToHost => (),
        Message::NewHost {
            ip,
            port,
            web_port,
            max_transfer,
        } => {
            info!("A new host has been promoted. Reconnecting to new host");
            repeat_except_for_client(
                client_id,
                server,
                &Message::NewHost {
                    ip,
                    port,
                    web_port,
                    max_transfer,
                },
            );
            server.disconnect_all();
            cmd.add(move |world: &mut World| {
                world.remove_resource::<NetcodeServerTransport>();
                info!("Creating a new client connection to new host...");
                world.insert_resource(create_client(ip, port));
            });
            send_promoted_event.send(PromotedToClient {});
            next_promotion_state.set(PromotionState::PromotedToClient);
        }
        Message::RequestInitialSync => {
            debug!("Sending initial sync to client id: {}", client_id);
            cmd.add(move |world: &mut World| send_initial_sync(client_id, world));
        }
    }
}

fn repeat_except_for_client(
    msg_client_id: bevy_renet::renet::ClientId,
    server: &mut RenetServer,
    msg: &Message,
) {
    for client_id in server.clients_id().into_iter() {
        if client_id == msg_client_id {
            continue;
        }
        server.send_message(
            client_id,
            DefaultChannel::ReliableOrdered,
            bincode::serialize(msg).unwrap(),
        );
    }
}
