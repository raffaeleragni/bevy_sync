mod assert;
mod setup;

use bevy_sync::SyncMark;
use serial_test::serial;
use setup::{MySynched, TestEnv, TestRun};

#[test]
#[serial]
fn test_initial_world_sync_sent_from_server() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world.spawn(SyncMark {}).id();

            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });

            1
        },
        TestRun::no_setup,
        |env, entity_count: u32, _| {
            assert::initial_sync_for_client_happened(
                &mut env.server,
                &mut env.clients[0],
                entity_count,
            );
            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_client(&mut env.clients[0]);
        },
    );
}

#[test]
#[serial]
fn test_init_sync_multiple_clients() {
    TestRun::default().run(
        3,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world.spawn(SyncMark {}).id();

            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });

            1
        },
        TestRun::no_setup,
        |env: &mut TestEnv, entity_count: u32, _| {
            for capp in &mut env.clients {
                assert::initial_sync_for_client_happened(&mut env.server, capp, entity_count);
            }

            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_clients(&mut env.clients);
        },
    );
}
