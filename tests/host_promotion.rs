use bevy::app::App;
use bevy_renet::renet::{transport::NetcodeServerTransport, RenetServer};
use bevy_sync::{PromoteToHostEvent, SyncConnectionParameters, SyncMark};
use serial_test::serial;
use setup::{TestEnv, TestRun};

mod assert;
mod setup;

#[test]
#[serial]
fn test_host_promotion_with_one_client() {
    TestRun::default().run(
        2,
        TestRun::no_pre_setup,
        |env| {
            // need this because the tests run on the same machine and promotion won't advance between
            // occupied server releasing connection on same port and client becoming server on same port
            alter_connection_port(env);
            assert_server_is_host(env);
            send_promotion_event(env);
            env.update(10);
            assert_one_client_is_host(env);
            env.server.world.spawn(SyncMark {});
            1
        },
        assert::entities_in_sync,
    );
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
