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
            "{:?} received EntitySpawn {{ id: {} }}",
            from,
            id
        ),
        Message::EntityParented {
            entity_id: eid,
            parent_id: pid,
        } => debug!(
            "{:?} received EntityParented {{ eid: {}, pid: {} }}",
            from,
            eid,
            pid,
        ),
        Message::EntityDelete { id } => debug!(
            "{:?} received EntityDelete {{ id: {} }}",
            from,
            id,
        ),
        Message::ComponentUpdated { id, name, data: _ } => {
            debug!(
                "{:?} received ComponentUpdated {{ id: {}, name: {} }}",
                from,
                id,
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
        Message::PromoteToHost => debug!("{:?} received PromoteToHost", from),
        Message::NewHost {
            ip,
            port,
            web_port,
            max_transfer,
        } => debug!(
                "{:?} received NewHost {{ ip: {} }} {{ port: {} }} {{ web_port: {} }} {{ max_transfer: {} }}",
                from, ip, port, web_port, max_transfer),
        Message::RequestInitialSync => debug!("Received a request for initial sync from client_id: {:?}", from)
    }
}
