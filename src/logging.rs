use bevy::prelude::*;

use crate::proto::Message;

#[derive(Debug)]
pub(crate) enum Who {
    Server,
    Client,
}

pub(crate) fn log_message_received(from: Who, message: &Message) {
    match message {
        Message::EntitySpawn { id } => debug!(
            "{:?} received EntitySpawn {{ id: {}v{} }}",
            from,
            id.index(),
            id.generation()
        ),
        Message::EntityParented {
            server_entity_id: eid,
            server_parent_id: pid,
        } => debug!(
            "{:?} received EntityParented {{ eid: {}v{}, pid: {}v{} }}",
            from,
            eid.index(),
            eid.generation(),
            pid.index(),
            pid.generation()
        ),
        Message::EntitySpawnBack {
            server_entity_id: sid,
            client_entity_id: cid,
        } => debug!(
            "{:?} received EntitySpawnBack {{sid: {}v{}, cid: {}v{}",
            from,
            sid.index(),
            sid.generation(),
            cid.index(),
            cid.generation()
        ),
        Message::EntityDelete { id } => debug!(
            "{:?} received EntityDelete {{ id: {}v{} }}",
            from,
            id.index(),
            id.generation()
        ),
        Message::ComponentUpdated { id, name, data: _ } => {
            debug!(
                "{:?} received ComponentUpdated {{ id: {}v{}, name: {} }}",
                from,
                id.index(),
                id.generation(),
                name
            )
        }
        Message::StandardMaterialUpdated { id, material: _ } => {
            debug!(
                "{:?} received StandardMaterialUpdated {{ uuid: {} }}",
                from, id
            )
        }
        Message::MeshUpdated { id, url } => {
            debug!(
                "{:?} received MeshUpdated {{ uuid: {} }} {{ url: {} }}",
                from, id, url
            )
        }
        Message::ImageUpdated { id, url } => {
            debug!(
                "{:?} received ImageUpdated {{ uuid: {} }} {{ url: {} }}",
                from, id, url
            )
        }
    }
}
