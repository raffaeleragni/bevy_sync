use bevy::{app::App, ecs::entity::Entity};
use bevy_renet::renet::{
    transport::{NetcodeClientTransport, NetcodeServerTransport},
    RenetServer,
};
use bevy_sync::{PromoteToHostEvent, SyncConnectionParameters, SyncMark};
use serial_test::serial;
use setup::{MySynched, TestEnv, TestRun};

use crate::assert::get_first_entity_component;

mod assert;
mod setup;

#[test]
#[serial]
fn test_host_promotion_with_one_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            let e_id = setup_and_check_sync(env);

            // need this because the tests run on the same machine and promotion won't advance between
            // occupied server releasing connection on same port and client becoming server on same port
            alter_connection_port(env);
            assert_server_is_host(env);
            send_promotion_event(env);
            env.update(10);
            assert_all_are_connected(env);
            assert_only_one_client_is_host(env);

            env.server
                .world_mut()
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 7;
        },
        |env, _, _| {
            let comp = get_first_entity_component::<MySynched>(env.server.world_mut()).unwrap();
            assert_eq!(comp.value, 7);
            assert!(!env.clients.is_empty());
            for c in env.clients.iter_mut() {
                let world = c.world_mut();
                let comp = get_first_entity_component::<MySynched>(world).unwrap();
                assert_eq!(comp.value, 7);
            }
        },
    );
}

#[ignore = "Still not achieved on multi clients"]
#[test]
#[serial]
fn test_host_promotion_with_more_clients() {
    TestRun::default().run(
        3,
        TestRun::no_pre_setup,
        |env| {
            let e_id = setup_and_check_sync(env);

            // need this because the tests run on the same machine and promotion won't advance between
            // occupied server releasing connection on same port and client becoming server on same port
            alter_connection_port(env);
            assert_server_is_host(env);
            send_promotion_event(env);
            env.update(10);
            assert_all_are_connected(env);
            assert_only_one_client_is_host(env);

            env.server
                .world_mut()
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 7;
        },
        |env, _, _| {
            let comp = get_first_entity_component::<MySynched>(env.server.world_mut()).unwrap();
            assert_eq!(comp.value, 7);
            assert!(!env.clients.is_empty());
            for c in env.clients.iter_mut() {
                let world = c.world_mut();
                let comp = get_first_entity_component::<MySynched>(world).unwrap();
                assert_eq!(comp.value, 7);
            }
        },
    );
}

fn setup_and_check_sync(env: &mut TestEnv) -> Entity {
    env.setup_registration::<MySynched>();
    let e_id = env.server.world_mut().spawn(SyncMark {}).id();
    env.update(4);
    let mut e = env.server.world_mut().entity_mut(e_id);
    e.insert(MySynched { value: 1 });
    env.update(4);
    assert!(!env.clients.is_empty());
    for c in env.clients.iter_mut() {
        let world = c.world_mut();
        let comp = get_first_entity_component::<MySynched>(world).unwrap();
        assert_eq!(comp.value, 1);
    }
    env.server
        .world_mut()
        .entity_mut(e_id)
        .get_mut::<MySynched>()
        .unwrap()
        .value = 2;
    env.update(4);
    for c in env.clients.iter_mut() {
        let world = c.world_mut();
        let comp = get_first_entity_component::<MySynched>(world).unwrap();
        assert_eq!(comp.value, 2);
    }

    e_id
}

fn alter_connection_port(env: &mut TestEnv) {
    let mut i = 1;
    increment_port(&mut env.server, i);
    for c in env.clients.iter_mut() {
        i += 1;
        increment_port(c, i);
    }
}

fn increment_port(app: &mut App, i: u16) {
    app.world_mut()
        .resource_mut::<SyncConnectionParameters>()
        .port += i;
}

fn send_promotion_event(env: &mut TestEnv) {
    let server = env.server.world_mut().resource_mut::<RenetServer>();
    let event = PromoteToHostEvent {
        id: server.clients_id().first().unwrap().to_owned(),
    };
    env.server.world_mut().send_event(event);
}

fn assert_server_is_host(env: &TestEnv) {
    assert!(is_host(&env.server));
    for c in env.clients.iter() {
        assert!(!is_host(c));
    }
}

fn assert_only_one_client_is_host(env: &TestEnv) {
    assert!(!is_host(&env.server));
    let mut found = false;
    for c in env.clients.iter() {
        if is_host(c) {
            found = true;
        }
    }
    assert!(found);
}

fn is_host(app: &App) -> bool {
    app.world()
        .get_resource::<NetcodeServerTransport>()
        .is_some()
}

fn is_client(app: &App) -> bool {
    app.world()
        .get_resource::<NetcodeClientTransport>()
        .is_some()
}

fn assert_all_are_connected(env: &TestEnv) {
    let app = &env.server;
    assert!(is_host(app) || is_client(app));
    for app in env.clients.iter() {
        assert!(is_host(app) || is_client(app));
    }
}
