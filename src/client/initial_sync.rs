use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient};

use crate::{full_sync::build_full_sync, proto::Message};

pub(crate) fn send_initial_sync(world: &mut World) {
    info!("Sending initial sync to server");
    // exclusive access to world while looping through all objects, this can be blocking/freezing for the server
    let mut initial_sync = match build_full_sync(world) {
        Ok(initial_sync) => initial_sync,
        Err(err) => {
            warn!("Failed initial sync to server because {}", err);
            return;
        }
    };
    let mut client = world.resource_mut::<RenetClient>();
    debug!("Initial sync(client) size: {}", initial_sync.len());
    for msg in initial_sync.drain(..) {
        let Ok(msg_bin) = bincode::serialize(&msg) else {
            warn!("Could not deserialize {:?}", msg);
            continue;
        };
        client.send_message(DefaultChannel::ReliableOrdered, msg_bin);
    }
    let Ok(msg_bin) = bincode::serialize(&Message::FinishedInitialSync) else {
        warn!("Could not deserialize FinishedInitialSync");
        return;
    };
    client.send_message(DefaultChannel::ReliableOrdered, msg_bin);
}
