use crate::{
    logging::{log_message_received, Who},
    networking::assets::SyncAssetTransfer,
    proto::SyncAssetType,
};

use super::*;

pub(crate) fn poll_for_messages(
    mut commands: Commands,
    mut track: ResMut<SyncTrackerRes>,
    mut sync_assets: ResMut<SyncAssetTransfer>,
    mut client: ResMut<RenetClient>,
) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        let deser_message = bincode::deserialize(&message).unwrap();
        client_received_a_message(deser_message, &mut track, &mut sync_assets, &mut commands);
    }
}

fn client_received_a_message(
    msg: Message,
    track: &mut ResMut<SyncTrackerRes>,
    sync_assets: &mut ResMut<SyncAssetTransfer>,
    cmd: &mut Commands,
) {
    log_message_received(Who::Client, &msg);
    match msg {
        Message::EntitySpawn { id } => {
            if let Some(e_id) = track.server_to_client_entities.get(&id) {
                if cmd.get_entity(*e_id).is_some() {
                    return;
                }
            }
            let e_id = cmd
                .spawn(SyncUp {
                    server_entity_id: id,
                })
                .id();
            // Need to update the map right away or else adjacent messages won't see each other entity
            track.server_to_client_entities.insert(id, e_id);
        }
        Message::EntitySpawnBack {
            server_entity_id: id,
            client_entity_id: back_id,
        } => {
            if let Some(mut e) = cmd.get_entity(back_id) {
                e.remove::<SyncMark>().insert(SyncUp {
                    server_entity_id: id,
                });
            }
        }
        Message::EntityParented {
            server_entity_id: e_id,
            server_parent_id: p_id,
        } => {
            let Some(&c_e_id) = track.server_to_client_entities.get(&e_id) else {
                return;
            };
            let Some(&c_p_id) = track.server_to_client_entities.get(&p_id) else {
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
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {
                return;
            };
            let Some(mut e) = cmd.get_entity(e_id) else {
                return;
            };
            e.despawn();
        }
        Message::ComponentUpdated { id, name, data } => {
            let Some(&e_id) = track.server_to_client_entities.get(&id) else {
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
    }
}
