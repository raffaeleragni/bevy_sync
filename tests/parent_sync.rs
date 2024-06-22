mod assert;
mod setup;

use bevy::prelude::*;
use bevy_sync::{SyncEntity, SyncMark};
use serial_test::serial;
use setup::{TestEnv, TestRun};
use uuid::Uuid;

use crate::assert::find_entity_with_server_id;

#[test]
#[serial]
fn test_entity_parent_is_transferred_from_server() {
    TestRun::default().run(
        1,
        |_| {},
        |env: &mut TestEnv| {
            let e1 = env.server.world.spawn(SyncMark {}).id();
            let e2 = env.server.world.spawn(SyncMark {}).id();

            env.update(3);

            env.server.world.entity_mut(e1).add_child(e2);
            let server_e_id1 = env
                .server
                .world
                .entity(e1)
                .get::<SyncEntity>()
                .unwrap()
                .uuid;
            let server_e_id2 = env
                .server
                .world
                .entity(e2)
                .get::<SyncEntity>()
                .unwrap()
                .uuid;
            (server_e_id1, server_e_id2)
        },
        |env: &mut TestEnv, _, entities: (Uuid, Uuid)| {
            for capp in &mut env.clients {
                let parent = find_entity_with_server_id(capp, entities.0)
                    .expect("Parent not found on client");
                let child = find_entity_with_server_id(capp, entities.1)
                    .expect("Children not found on client");
                assert_eq!(
                    capp.world
                        .entity(parent)
                        .get::<Children>()
                        .expect("Parent has no children component")
                        .iter()
                        .filter(|e| **e == child)
                        .count(),
                    1
                );
                assert_eq!(
                    capp.world
                        .entity(child)
                        .get::<Parent>()
                        .expect("Child has no parent component")
                        .get(),
                    parent
                );
            }

            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_clients(&mut env.clients);
        },
    );
}

#[test]
#[serial]
fn test_entity_parent_is_transferred_from_client() {
    TestRun::default().run(
        1,
        |_| {},
        |env: &mut TestEnv| {
            let e_id1 = env.clients[0].world.spawn(SyncMark {}).id();
            let e_id2 = env.clients[0].world.spawn(SyncMark {}).id();

            env.update(4);

            env.clients[0].world.entity_mut(e_id1).add_child(e_id2);

            env.update(4);

            let e1 = &env.clients[0].world.entity(e_id1);
            let server_e1 = e1.get::<SyncEntity>().unwrap().uuid;
            let e2 = &env.clients[0].world.entity(e_id2);
            let server_e2 = e2.get::<SyncEntity>().unwrap().uuid;

            (server_e1, server_e2)
        },
        |env: &mut TestEnv, _, entities: (Uuid, Uuid)| {
            let parent = entities.0;
            let child = entities.1;
            let parent = find_entity_with_server_id(&mut env.server, parent).unwrap();
            let child = find_entity_with_server_id(&mut env.server, child).unwrap();
            let parent = env.server.world.entity(parent);
            let child = env.server.world.entity(child);
            assert_eq!(
                parent
                    .get::<Children>()
                    .expect("Parent has no children component")
                    .iter()
                    .filter(|e| **e == child.id())
                    .count(),
                1
            );
            assert_eq!(
                child
                    .get::<Parent>()
                    .expect("Child has no parent component")
                    .get(),
                parent.id()
            );

            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_clients(&mut env.clients);
        },
    );
}
