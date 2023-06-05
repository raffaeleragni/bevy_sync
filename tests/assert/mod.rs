use bevy::prelude::App;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};
use bevy_sync::{SyncDown, SyncUp};

use crate::setup::{MySynched, TestEnv};

#[allow(dead_code)]
pub(crate) fn entities_in_sync<T>(env: &mut TestEnv, _: T, entity_count: u32) {
    for c in &mut env.clients {
        let mut count_check = 0;
        for e in c.world.query::<&SyncUp>().iter(&c.world) {
            assert!(env.server.world.entities().contains(e.server_entity_id));
            env.server
                .world
                .entity(e.server_entity_id)
                .get::<SyncDown>();
            count_check += 1;
        }
        assert_eq!(count_check, entity_count);
    }
}

#[allow(dead_code)]
pub(crate) fn no_messages_left_for_server(s: &mut App) {
    let mut server = s.world.resource_mut::<RenetServer>();
    for client_id in server.clients_id().into_iter() {
        assert!(server
            .receive_message(client_id, DefaultChannel::ReliableOrdered)
            .is_none());
    }
}

#[allow(dead_code)]
pub(crate) fn no_messages_left_for_clients(cs: &mut Vec<App>) {
    for c in cs {
        no_messages_left_for_client(c);
    }
}

#[allow(dead_code)]
pub(crate) fn no_messages_left_for_client(c: &mut App) {
    let mut client = c.world.resource_mut::<RenetClient>();
    assert!(client
        .receive_message(DefaultChannel::ReliableOrdered)
        .is_none());
}

#[allow(dead_code)]
pub(crate) fn initial_sync_for_client_happened(s: &mut App, c: &mut App, entity_count: u32) {
    let mut count_check = 0;
    for (e, c) in c.world.query::<(&SyncUp, &MySynched)>().iter(&c.world) {
        assert!(s.world.entities().contains(e.server_entity_id));
        assert_eq!(c.value, 7);
        s.world.entity(e.server_entity_id).get::<SyncDown>();
        count_check += 1;
    }
    assert_eq!(count_check, entity_count);
}
