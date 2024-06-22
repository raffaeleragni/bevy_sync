use bevy_sync::{SyncEntity, SyncMark};
use serial_test::serial;
use setup::TestRun;
use uuid::Uuid;

use crate::assert::find_entity_with_server_id;

mod assert;
mod setup;

#[test]
#[serial]
fn test_one_entity_spawned_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.server.world.spawn(SyncMark {});
            1
        },
        assert::entities_in_sync,
    );
}

#[test]
#[serial]
fn test_one_entity_spawned_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.clients[0].world.spawn(SyncMark {});
            1
        },
        assert::entities_in_sync,
    );
}

#[test]
#[serial]
fn test_more_entities_spawned_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.server.world.spawn(SyncMark {});
            env.server.world.spawn(SyncMark {});
            env.server.world.spawn(SyncMark {});
            3
        },
        assert::entities_in_sync,
    );
}

#[test]
#[serial]
fn test_more_entities_spawned_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.clients[0].world.spawn(SyncMark {});
            env.clients[0].world.spawn(SyncMark {});
            env.clients[0].world.spawn(SyncMark {});
            3
        },
        assert::entities_in_sync,
    );
}

#[test]
#[serial]
fn test_entity_deleted_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(3);
            env.server.world.entity_mut(e_id).despawn();
            0
        },
        assert::entities_in_sync,
    );
}

#[test]
#[serial]
fn test_entity_deleted_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            let e_id = env.clients[0].world.spawn(SyncMark {}).id();
            env.update(5);
            let e = env.clients[0].world.entity_mut(e_id);
            let server_e_id = e.get::<SyncEntity>().unwrap().uuid;
            e.despawn();
            server_e_id
        },
        |env, _, id: Uuid| {
            let e = find_entity_with_server_id(&mut env.server, id);
            assert!(e.is_none());
        },
    );
}
