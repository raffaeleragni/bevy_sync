use bevy::{app::App, ecs::entity::Entity};
use bevy_renet::renet::{transport::NetcodeServerTransport, RenetServer};
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
        2,
        TestRun::no_pre_setup,
        |env| {
            let e_id = setup_and_check_sync(env);

            // need this because the tests run on the same machine and promotion won't advance between
            // occupied server releasing connection on same port and client becoming server on same port
            alter_connection_port(env);
            assert_server_is_host(env);
            send_promotion_event(env);
            env.update(10);
            assert_one_client_is_host(env);

            env.server
                .world
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 7;
        },
        |env, _, _| {
            let comp = get_first_entity_component::<MySynched>(&mut env.clients[0]).unwrap();
            assert_eq!(comp.value, 7);
            let comp = get_first_entity_component::<MySynched>(&mut env.clients[1]).unwrap();
            assert_eq!(comp.value, 7);
        },
    );
}

fn setup_and_check_sync(env: &mut TestEnv) -> Entity {
    env.setup_registration::<MySynched>();
    let e_id = env.server.world.spawn(SyncMark {}).id();
    env.update(4);
    let mut e = env.server.world.entity_mut(e_id);
    e.insert(MySynched { value: 1 });
    env.update(4);
    let comp = get_first_entity_component::<MySynched>(&mut env.clients[0]).unwrap();
    assert_eq!(comp.value, 1);
    let comp = get_first_entity_component::<MySynched>(&mut env.clients[1]).unwrap();
    assert_eq!(comp.value, 1);
    env.server
        .world
        .entity_mut(e_id)
        .get_mut::<MySynched>()
        .unwrap()
        .value = 2;
    env.update(4);
    let comp = get_first_entity_component::<MySynched>(&mut env.clients[0]).unwrap();
    assert_eq!(comp.value, 2);
    let comp = get_first_entity_component::<MySynched>(&mut env.clients[1]).unwrap();
    assert_eq!(comp.value, 2);

    e_id
}

fn alter_connection_port(env: &mut TestEnv) {
    increment_port(&mut env.server);
    for c in env.clients.iter_mut() {
        increment_port(c);
    }
}

fn increment_port(app: &mut App) {
    app.world.resource_mut::<SyncConnectionParameters>().port += 1;
}

fn send_promotion_event(env: &mut TestEnv) {
    let server = env.server.world.resource_mut::<RenetServer>();
    let event = PromoteToHostEvent {
        id: server.clients_id().first().unwrap().to_owned(),
    };
    env.server.world.send_event(event);
}

fn assert_server_is_host(env: &TestEnv) {
    assert!(is_host(&env.server));
    for c in env.clients.iter() {
        assert!(!is_host(c));
    }
}

fn assert_one_client_is_host(env: &TestEnv) {
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
    app.world.get_resource::<NetcodeServerTransport>().is_some()
}
