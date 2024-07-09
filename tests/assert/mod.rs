use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetServer};
use bevy_sync::SyncEntity;
use uuid::Uuid;

use crate::setup::{sample_image, sample_mesh, MySynched, TestEnv};

#[allow(dead_code)]
pub(crate) fn entities_in_sync<T>(env: &mut TestEnv, _: T, entity_count: u32) {
    for c in &mut env.clients {
        let mut count_check = 0;
        for e in c.world_mut().query::<&SyncEntity>().iter(c.world()) {
            for se in env
                .server
                .world_mut()
                .query::<&SyncEntity>()
                .iter(env.server.world())
            {
                if se.uuid == e.uuid {
                    count_check += 1;
                }
            }
        }
        assert_eq!(count_check, entity_count);
    }
}

#[allow(dead_code)]
pub(crate) fn no_messages_left_for_server(s: &mut App) {
    let mut server = s.world_mut().resource_mut::<RenetServer>();
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
    let mut client = c.world_mut().resource_mut::<RenetClient>();
    assert!(client
        .receive_message(DefaultChannel::ReliableOrdered)
        .is_none());
}

#[allow(dead_code)]
pub(crate) fn initial_sync_for_client_happened(s: &mut App, c: &mut App, entity_count: u32) {
    let mut count_check = 0;
    for (e, c) in c
        .world_mut()
        .query::<(&SyncEntity, &MySynched)>()
        .iter(c.world())
    {
        for se in s.world_mut().query::<&SyncEntity>().iter(s.world()) {
            if se.uuid == e.uuid {
                count_check += 1;
                assert_eq!(c.value, 7);
            }
        }
    }
    assert_eq!(count_check, entity_count);
}

#[allow(dead_code)]
pub(crate) fn count_entities_with_component<T: Component>(app: &mut App) -> u32 {
    let mut count = 0;
    for _ in app
        .world_mut()
        .query_filtered::<Entity, With<T>>()
        .iter(app.world())
    {
        count += 1;
    }
    count
}

#[allow(dead_code)]
pub(crate) fn count_entities_without_component<T: Component>(app: &mut App) -> u32 {
    let mut count = 0;
    for _ in app
        .world_mut()
        .query_filtered::<Entity, Without<T>>()
        .iter(app.world())
    {
        count += 1;
    }
    count
}

#[allow(dead_code)]
pub(crate) fn get_first_entity_component<T: Component>(app: &mut App) -> Option<&T> {
    app.world_mut().query::<&T>().iter(app.world()).next()
}

#[allow(dead_code)]
pub(crate) fn material_has_color(app: &mut App, id: AssetId<StandardMaterial>, color: Color) {
    let materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let material = materials.get(id).unwrap();
    assert_eq!(material.base_color, color);
}

#[allow(dead_code)]
pub(crate) fn assets_has_sample_mesh(app: &mut App, id: AssetId<Mesh>) {
    let meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    let mesh = meshes.get(id).unwrap();
    let sample = sample_mesh();
    assert_eq!(
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .get_bytes(),
        sample
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .get_bytes()
    );
}

#[allow(dead_code)]
pub(crate) fn assets_has_sample_image(app: &mut App, id: AssetId<Image>) {
    let images = app.world_mut().resource_mut::<Assets<Image>>();
    let image = images.get(id).unwrap();
    let sample = sample_image();
    assert_eq!(image.data, sample.data);
}

#[allow(dead_code)]
pub(crate) fn find_entity_with_server_id(app: &mut App, server_entity_id: Uuid) -> Option<Entity> {
    for (entity, sup) in app
        .world_mut()
        .query::<(Entity, &SyncEntity)>()
        .iter(app.world())
    {
        if sup.uuid == server_entity_id {
            return Some(entity);
        }
    }
    None
}
