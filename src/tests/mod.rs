mod setup;
use super::*;
use setup::TestEnv;

#[test]
fn test_connection_setup() {
    TestEnv::default().run(|_, _| {}, |_, _, _| {});
}

#[test]
fn test_entity_spawned_from_server() {
    TestEnv::default().run(
        |s: &mut App, _: &mut App| s.world.spawn(SyncDown::default()).id(),
        |_: &mut App, c: &mut App, id: Entity| {
            let mut empty = true;
            for e in c.world.query::<&SyncUp>().iter(&c.world) {
                assert_eq!(e.server_entity_id, id);
                empty = false;
            }
            assert!(!empty);
        },
    );
}

#[test]
fn test_entity_spawned_from_client() {
    TestEnv::default().run(
        |_: &mut App, c: &mut App| c.world.spawn(SyncEntitySpawnedFromClient {}).id(),
        |s: &mut App, c: &mut App, _: Entity| {
            let mut empty = true;
            for e in c.world.query::<&SyncUp>().iter(&c.world) {
                s.world.entities().get(e.server_entity_id).unwrap();
                empty = false;
            }
            assert!(!empty);
        },
    );
}
