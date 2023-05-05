use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};

use crate::proto::Message;

use super::SyncUp;

pub struct SendPlugin;

impl Plugin for SendPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(on_changed_sync_up);
    }
}

fn on_changed_sync_up(
    opt_server: Option<ResMut<RenetServer>>,
    opt_client: Option<ResMut<RenetClient>>,
    mut query: Query<(Entity, &mut SyncUp), Changed<SyncUp>>,
) {
    if let Some(server) = opt_server {
        send_updated_components_from_server(server, &mut query);
    }
    if let Some(client) = opt_client {
        send_updated_components_from_client(client, &mut query);
    }
}

fn send_updated_components_from_server(
    mut server: ResMut<RenetServer>,
    query: &mut Query<(Entity, &mut SyncUp), Changed<SyncUp>>,
) {
    for client_id in server.clients_id().into_iter() {
        for (id, sync) in query.iter_mut() {
            let msg = build_update_message(sync, id);
            server.send_message(client_id, DefaultChannel::Reliable, msg);
        }
    }
}

fn build_update_message(mut sync: Mut<SyncUp>, id: Entity) -> Vec<u8> {
    sync.changed = false;
    let msg = bincode::serialize(&Message::ComponentUpdated {
        id: id,
        type_id: 1u64,
        data: vec![],
    })
    .unwrap();
    msg
}

fn send_updated_components_from_client(
    mut client: ResMut<RenetClient>,
    query: &mut Query<(Entity, &mut SyncUp), Changed<SyncUp>>,
) {
    for (id, sync) in query.iter_mut() {
        let msg = build_update_message(sync, id);
        client.send_message(DefaultChannel::Reliable, msg);
    }
}
