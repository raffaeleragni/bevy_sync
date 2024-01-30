use bevy_renet::renet::ClientId;

use crate::{
    logging::{log_message_received, Who},
    networking::assets::SyncAssetTransfer,
    proto::SyncAssetType,
};

use super::*;

pub(crate) fn poll_for_messages(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut track: ResMut<SyncTrackerRes>,
    mut sync_assets: ResMut<SyncAssetTransfer>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let deser_message = bincode::deserialize(&message).unwrap();
            server_received_a_message(
                client_id,
                deser_message,
                &mut track,
                &mut sync_assets,
                &mut commands,
            );
        }
    }
}

fn server_received_a_message(
    client_id: ClientId,
    msg: Message,
    track: &mut ResMut<SyncTrackerRes>,
    sync_assets: &mut ResMut<SyncAssetTransfer>,
    cmd: &mut Commands,
) {
    log_message_received(Who::Server, &msg);
    match msg {
        Message::EntitySpawn { id } => {
            let e_id = cmd
                .spawn(SyncClientGeneratedEntity {
                    client_id,
                    client_entity_id: id,
                })
                .id();
            // Need to update the map right away or else adjacent messages won't see each other entity
            track.server_to_client_entities.insert(e_id, e_id);
        }
        Message::EntityParented {
            server_entity_id: e_id,
            server_parent_id: p_id,
        } => {
            cmd.add(move |world: &mut World| {
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
                        server_entity_id: e_id,
                        server_parent_id: p_id,
                    },
                );
            });
        }
        Message::EntityDelete { id } => {
            if let Some(mut e) = cmd.get_entity(id) {
                e.despawn();
            }
        }
        // This has no meaning on server side
        Message::EntitySpawnBack {
            server_entity_id: _,
            client_entity_id: _,
        } => {}
        Message::ComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {
                return;
            };
            cmd.add(move |world: &mut World| {
                let changed = SyncTrackerRes::apply_component_change_from_network(
                    e_id,
                    name.clone(),
                    &data,
                    world,
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
