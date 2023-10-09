use bevy::{asset::AssetIndex, prelude::*};
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

#[allow(dead_code)]
pub(crate) fn count_entities_with_component<T: Component>(app: &mut App) -> u32 {
    let mut count = 0;
    for _ in app
        .world
        .query_filtered::<Entity, With<T>>()
        .iter(&app.world)
    {
        count += 1;
    }
    count
}

#[allow(dead_code)]
pub(crate) fn count_entities_without_component<T: Component>(app: &mut App) -> u32 {
    let mut count = 0;
    for _ in app
        .world
        .query_filtered::<Entity, Without<T>>()
        .iter(&app.world)
    {
        count += 1;
    }
    count
}

#[allow(dead_code)]
pub(crate) fn get_first_entity_component<T: Component>(app: &mut App) -> Option<&T> {
    app.world.query::<&T>().iter(&app.world).next()
}

#[allow(dead_code)]
pub(crate) fn material_has_color(app: &mut App, id: AssetId<StandardMaterial>, color: Color) {
    let materials = app.world.resource_mut::<Assets<StandardMaterial>>();
    let material = materials.get(id).unwrap();
    assert_eq!(material.base_color, color);
}

#[allow(dead_code)]
pub(crate) fn assets_has_mesh(app: &mut App, id: AssetId<Mesh>) {
    let meshes = app.world.resource_mut::<Assets<Mesh>>();
    let _ = meshes.get(id).unwrap();
}

#[allow(dead_code)]
pub(crate) fn find_entity_with_server_id(
    app: &mut App,
    server_entity_id: Entity,
) -> Option<Entity> {
    for (entity, sup) in app
        .world
        .query_filtered::<(Entity, &SyncUp), With<SyncUp>>()
        .iter(&app.world)
    {
        if sup.server_entity_id == server_entity_id {
            return Some(entity);
        }
    }
    None
}
