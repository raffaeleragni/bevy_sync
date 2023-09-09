mod assert;
mod setup;

use bevy::prelude::{Entity, With, Without};
use bevy_sync::{SyncDown, SyncExclude, SyncMark, SyncUp};
use serial_test::serial;
use setup::{MyNonSynched, MySynched, TestEnv, TestRun};

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.server.world.spawn((SyncMark {}, MyNonSynched {}));
            0
        },
        |env, _, _| {
            let mut count_check = 0;
            for _ in env.clients[0]
                .world
                .query_filtered::<Entity, With<MyNonSynched>>()
                .iter(&env.clients[0].world)
            {
                count_check += 1;
            }
            assert_eq!(count_check, 0);
        },
    );
}

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.clients[0]
                .world
                .spawn((SyncMark {}, MyNonSynched {}))
                .id()
        },
        |env, _, _| {
            let mut found = false;
            for _ in env
                .server
                .world
                .query_filtered::<&SyncUp, Without<MyNonSynched>>()
                .iter(&env.server.world)
            {
                found = true;
            }
            assert!(!found);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            1
        },
        |env, _, entity_count: u32| {
            let mut count_check = 0;
            for (e, c) in env.clients[0]
                .world
                .query::<(&SyncUp, &MySynched)>()
                .iter(&env.clients[0].world)
            {
                assert!(env.server.world.entities().contains(e.server_entity_id));
                assert_eq!(c.value, 7);
                env.server
                    .world
                    .entity(e.server_entity_id)
                    .get::<SyncDown>();
                count_check += 1;
            }
            assert_eq!(count_check, entity_count);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server_then_changed() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(3);

            env.server
                .world
                .entity_mut(e_id)
                .insert(MySynched { value: 7 });
            env.update(3);

            env.server
                .world
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 3;
            env.update(3);

            1
        },
        |env, _, entity_count: u32| {
            let mut count_check = 0;
            for (e, c) in env.clients[0]
                .world
                .query::<(&SyncUp, &MySynched)>()
                .iter(&env.clients[0].world)
            {
                assert!(env.server.world.entities().contains(e.server_entity_id));
                assert_eq!(c.value, 3);
                env.server
                    .world
                    .entity(e.server_entity_id)
                    .get::<SyncDown>();
                count_check += 1;
            }
            assert_eq!(count_check, entity_count);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.clients[0].world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.clients[0].world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            let server_e_id = e.get::<SyncUp>().unwrap().server_entity_id;
            server_e_id
        },
        |env, _, id: Entity| {
            let e = env.server.world.get_entity(id).unwrap();
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 7);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client_then_changed() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.clients[0].world.spawn(SyncMark {}).id();
            env.update(3);

            let mut e = env.clients[0].world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            env.update(3);

            env.clients[0]
                .world
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 3;
            env.update(3);

            let server_e_id = env.clients[0]
                .world
                .entity_mut(e_id)
                .get::<SyncUp>()
                .unwrap()
                .server_entity_id;
            server_e_id
        },
        |env: &mut TestEnv, _, id: Entity| {
            let e = env.server.world.get_entity(id).unwrap();
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 3);
        },
    );
}

#[test]
#[serial]
fn exclusion_marked_will_not_be_synced() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env
                .server
                .world
                .spawn((SyncMark {}, SyncExclude::<MySynched>::default()))
                .id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            0
        },
        |env, _, _| {
            let mut count_check = 0;
            for _ in env.clients[0]
                .world
                .query_filtered::<Entity, With<MySynched>>()
                .iter(&env.clients[0].world)
            {
                count_check += 1;
            }
            assert_eq!(count_check, 0);
        },
    );
}
