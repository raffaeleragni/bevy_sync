mod setup;
use super::*;
use setup::TestEnv;

#[test]
fn test_connection_setup() {
    TestEnv::default().run(|_, _| {}, |_, _, _| {});
}

fn all_client_entities_are_in_sync(s: &mut App, c: &mut App, count: u32) {
    let mut count_check = 0;
    for e in c.world.query::<&SyncUp>().iter(&c.world) {
        s.world.entities().get(e.server_entity_id).unwrap();
        count_check += 1;
    }
    assert_eq!(count_check, count);
}

#[test]
fn test_one_entity_spawned_from_server() {
    TestEnv::default().run(
        |s: &mut App, _: &mut App| {
            s.world.spawn(SyncDown::default());
            1
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
fn test_one_entity_spawned_from_client() {
    TestEnv::default().run(
        |_: &mut App, c: &mut App| {
            c.world.spawn(SyncEntitySpawnedFromClient {});
            1
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
fn test_more_entities_spawned_from_server() {
    TestEnv::default().run(
        |s: &mut App, _: &mut App| {
            s.world.spawn(SyncDown::default());
            s.world.spawn(SyncDown::default());
            s.world.spawn(SyncDown::default());
            3
        },
        all_client_entities_are_in_sync,
    );
}

#[test]
fn test_more_entities_spawned_from_client() {
    TestEnv::default().run(
        |_: &mut App, c: &mut App| {
            c.world.spawn(SyncEntitySpawnedFromClient {});
            c.world.spawn(SyncEntitySpawnedFromClient {});
            c.world.spawn(SyncEntitySpawnedFromClient {});
            3
        },
        all_client_entities_are_in_sync,
    );
}
