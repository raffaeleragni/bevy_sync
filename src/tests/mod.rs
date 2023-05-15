mod setup;
use super::*;
use setup::TestEnv;

#[test]
fn test_connection_setup() {
    TestEnv::default().run(|_, _| {}, |_, _, _| {});
}

fn all_client_entities_are_in_sync(s: &mut App, c: &mut App, entity_count: u32) {
    let mut count_check = 0;
    for e in c.world.query::<&SyncUp>().iter(&c.world) {
        assert!(s.world.entities().contains(e.server_entity_id));
        s.world.entity(e.server_entity_id).get::<SyncDown>();
        count_check += 1;
    }
    assert_eq!(count_check, entity_count);
}

#[test]
fn test_one_entity_spawned_from_server() {
    TestEnv::default().run(
        |s: &mut App, _: &mut App| {
            s.world.spawn(SyncMark {});
            1
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
fn test_one_entity_spawned_from_client() {
    TestEnv::default().run(
        |_: &mut App, c: &mut App| {
            c.world.spawn(SyncMark {});
            1
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
fn test_more_entities_spawned_from_server() {
    TestEnv::default().run(
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
fn test_more_entities_spawned_from_client() {
    TestEnv::default().run(
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
fn test_entity_deleted_from_server() {
    TestEnv::default().run(
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
