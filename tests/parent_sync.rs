mod assert;
mod setup;

use bevy::prelude::{App, BuildWorldChildren, Children, Entity, Parent, With};
use bevy_sync::{SyncMark, SyncUp};
use serial_test::serial;
use setup::{TestEnv, TestRun};

fn find_entity_with_server_id(c: &mut App, e_id: Entity) -> Option<Entity> {
    for (c_e, sup) in c
        .world
        .query_filtered::<(Entity, &SyncUp), With<SyncUp>>()
        .iter(&c.world)
    {
        if sup.server_entity_id == e_id {
            return Some(c_e);
        }
    }
    None
}

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
            (e1, e2)
        },
        |env: &mut TestEnv, _, entities: (Entity, Entity)| {
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
            let server_e1 = e1.get::<SyncUp>().unwrap().server_entity_id;
            let e2 = &env.clients[0].world.entity(e_id2);
            let server_e2 = e2.get::<SyncUp>().unwrap().server_entity_id;

            (server_e1, server_e2)
        },
        |env: &mut TestEnv, _, entities: (Entity, Entity)| {
            let parent = entities.0;
            let child = entities.1;
            assert_eq!(
                env.server
                    .world
                    .entity(parent)
                    .get::<Children>()
                    .expect("Parent has no children component")
                    .iter()
                    .filter(|e| **e == child)
                    .count(),
                1
            );
            assert_eq!(
                env.server
                    .world
                    .entity(child)
                    .get::<Parent>()
                    .expect("Child has no parent component")
                    .get(),
                parent
            );

            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_clients(&mut env.clients);
        },
    );
}

#[test]
#[serial]
fn test_mesh_transferred_from_server_to_client() {
    TestRun::default().run(1, |_| {}, |_| {}, |_, _, _| {});
}
