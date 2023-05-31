mod setup;

use crate::prelude::SyncComponent;

use self::setup::TestRun;

use super::*;
use bevy::reflect::ReflectFromReflect;
use bevy_renet::renet::DefaultChannel;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use setup::TestEnv;

fn all_client_entities_are_in_sync<T>(s: &mut App, c: &mut App, _: T, entity_count: u32) {
    let mut count_check = 0;
    for e in c.world.query::<&SyncUp>().iter(&c.world) {
        assert!(s.world.entities().contains(e.server_entity_id));
        s.world.entity(e.server_entity_id).get::<SyncDown>();
        count_check += 1;
    }
    assert_eq!(count_check, entity_count);
}

#[test]
#[serial]
fn test_one_entity_spawned_from_server() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, _: &mut App| {
            s.world.spawn(SyncMark {});
            1
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
#[serial]
fn test_one_entity_spawned_from_client() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |_: &mut App, c: &mut App| {
            c.world.spawn(SyncMark {});
            1
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
#[serial]
fn test_more_entities_spawned_from_server() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, _: &mut App| {
            s.world.spawn(SyncMark {});
            s.world.spawn(SyncMark {});
            s.world.spawn(SyncMark {});
            3
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
#[serial]
fn test_more_entities_spawned_from_client() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |_: &mut App, c: &mut App| {
            c.world.spawn(SyncMark {});
            c.world.spawn(SyncMark {});
            c.world.spawn(SyncMark {});
            3
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
#[serial]
fn test_entity_deleted_from_server() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, c: &mut App| {
            let e_id = s.world.spawn(SyncMark {}).id();
            s.update();
            c.update();
            s.update();
            c.update();
            s.world.entity_mut(e_id).despawn();
            0
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
#[serial]
fn test_entity_deleted_from_client() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, c: &mut App| {
            let e_id = c.world.spawn(SyncMark {}).id();
            s.update();
            c.update();
            s.update();
            c.update();
            s.update();
            c.update();
            s.update();
            c.update();
            let e = c.world.entity_mut(e_id);
            let server_e_id = e.get::<SyncUp>().unwrap().server_entity_id;
            e.despawn();
            server_e_id
        },
        |s: &mut App, _: &mut App, _, id: Entity| {
            assert!(s.world.get_entity(id).is_none());
        },
    );
}

#[derive(Component)]
struct MyNonSynched;

#[derive(Component, Reflect, FromReflect, Default, PartialEq, Serialize, Deserialize, Debug)]
#[reflect(Component, FromReflect)]
struct MySynched {
    value: i32,
}

fn setup_registration(a: &mut App) {
    a.sync_component::<MySynched>();
}

fn setup_registrations(apps: &mut Vec<App>) {
    for a in apps {
        a.sync_component::<MySynched>();
    }
}

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_server() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, _: &mut App| {
            s.world.spawn((SyncMark {}, MyNonSynched {}));
            1
        },
        |s: &mut App, c: &mut App, _, entity_count: u32| {
            let mut count_check = 0;
            for e in c
                .world
                .query_filtered::<&SyncUp, Without<MyNonSynched>>()
                .iter(&c.world)
            {
                assert!(s.world.entities().contains(e.server_entity_id));
                s.world.entity(e.server_entity_id).get::<SyncDown>();
                count_check += 1;
            }
            assert_eq!(count_check, entity_count);
        },
    );
}

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_client() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |_: &mut App, c: &mut App| c.world.spawn((SyncMark {}, MyNonSynched {})).id(),
        |s: &mut App, _: &mut App, _, _| {
            let mut found = false;
            for _ in s
                .world
                .query_filtered::<&SyncUp, Without<MyNonSynched>>()
                .iter(&s.world)
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
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, c: &mut App| {
            setup_registration(s);
            setup_registration(c);
            let e_id = s.world.spawn(SyncMark {}).id();
            s.update();
            c.update();
            s.update();
            c.update();
            s.update();
            c.update();
            s.update();
            c.update();
            let mut e = s.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            1
        },
        |s: &mut App, c: &mut App, _, entity_count: u32| {
            let mut count_check = 0;
            for (e, c) in c.world.query::<(&SyncUp, &MySynched)>().iter(&c.world) {
                assert!(s.world.entities().contains(e.server_entity_id));
                assert_eq!(c.value, 7);
                s.world.entity(e.server_entity_id).get::<SyncDown>();
                count_check += 1;
            }
            assert_eq!(count_check, entity_count);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client() {
    TestEnv::default().run_with_single_client(
        |_, _| {},
        |s: &mut App, c: &mut App| {
            setup_registration(s);
            setup_registration(c);
            let e_id = c.world.spawn(SyncMark {}).id();
            s.update();
            c.update();
            s.update();
            c.update();
            s.update();
            c.update();
            s.update();
            c.update();
            let mut e = c.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            let server_e_id = e.get::<SyncUp>().unwrap().server_entity_id;
            server_e_id
        },
        |s: &mut App, _: &mut App, _, id: Entity| {
            let e = s.world.get_entity(id).unwrap();
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 7);
        },
    );
}

fn setup_initial_sync_on_server(s: &mut App) -> u32 {
    setup_registration(s);

    let e_id = s.world.spawn(SyncMark {}).id();

    let mut e = s.world.entity_mut(e_id);
    e.insert(MySynched { value: 7 });

    1
}

fn verify_initial_sync_for_client(s: &mut App, c: &mut App, entity_count: u32) {
    let mut count_check = 0;
    for (e, c) in c.world.query::<(&SyncUp, &MySynched)>().iter(&c.world) {
        assert!(s.world.entities().contains(e.server_entity_id));
        assert_eq!(c.value, 7);
        s.world.entity(e.server_entity_id).get::<SyncDown>();
        count_check += 1;
    }
    assert_eq!(count_check, entity_count);
}

fn verify_no_messages_left_for_server(s: &mut App) {
    let mut server = s.world.resource_mut::<RenetServer>();
    for client_id in server.clients_id().into_iter() {
        assert!(server
            .receive_message(client_id, DefaultChannel::ReliableOrdered)
            .is_none());
    }
}

fn verify_no_messages_left_for_clients(cs: &mut Vec<App>) {
    for c in cs {
        verify_no_messages_left_for_client(c);
    }
}

fn verify_no_messages_left_for_client(c: &mut App) {
    let mut client = c.world.resource_mut::<RenetClient>();
    assert!(client
        .receive_message(DefaultChannel::ReliableOrdered)
        .is_none());
}

#[test]
#[serial]
fn test_initial_world_sync_sent_from_server() {
    TestEnv::default().run_with_single_client(
        |s: &mut App, c: &mut App| {
            setup_registration(c);
            setup_initial_sync_on_server(s)
        },
        |_, _| {},
        |s: &mut App, c: &mut App, entity_count: u32, _| {
            verify_initial_sync_for_client(s, c, entity_count);
            verify_no_messages_left_for_server(s);
            verify_no_messages_left_for_client(c);
        },
    );
}

#[test]
#[serial]
fn test_init_sync_multiple_clients() {
    TestEnv::default().run_with_multiple_clients(
        3,
        |env: &mut TestRun| {
            setup_registrations(&mut env.clients);
            setup_initial_sync_on_server(&mut env.server)
        },
        |_| {},
        |env: &mut TestRun, entity_count: u32, _| {
            for capp in &mut env.clients {
                verify_initial_sync_for_client(&mut env.server, capp, entity_count);
            }

            verify_no_messages_left_for_server(&mut env.server);
            verify_no_messages_left_for_clients(&mut env.clients);
        },
    );
}
