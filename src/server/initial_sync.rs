use bevy::prelude::*;
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};

use crate::full_sync::build_full_sync;

pub(crate) fn send_initial_sync(client_id: ClientId, world: &mut World) {
    info!("Sending initial sync to client id {}", client_id);
    // exclusive access to world while looping through all objects, this can be blocking/freezing for the server
    let mut initial_sync = match build_full_sync(world) {
        Ok(initial_sync) => initial_sync,
        Err(err) => {
            warn!(
                "Failed initial sync to client id {} because {}",
                client_id, err
            );
            return;
        }
    };
    let mut server = world.resource_mut::<RenetServer>();
    debug!("Initial sync size: {}", initial_sync.len());
    for msg in initial_sync.drain(..) {
        let Ok(msg_bin) = bincode::serialize(&msg) else {
            warn!("Could not deserialize {:?}", msg);
            continue;
        };
        server.send_message(client_id, DefaultChannel::ReliableOrdered, msg_bin);
    }
}
