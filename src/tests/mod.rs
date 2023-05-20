mod setup;
use crate::data::SyncComponent;

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

#[test]
fn test_entity_deleted_from_client() {
    TestEnv::default().run(
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
        |s: &mut App, _: &mut App, id: Entity| {
            assert!(s.world.get_entity(id).is_none());
        },
    );
}

#[derive(Component)]
pub struct MyNonSynched;

#[derive(Component)]
pub struct MySynched;

#[test]
fn test_non_marked_component_is_not_transferred_from_server() {
    TestEnv::default().run(
        |s: &mut App, _: &mut App| {
            s.world.spawn((SyncMark {}, MyNonSynched {}));
            1
        },
        |s: &mut App, c: &mut App, entity_count: u32| {
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
fn test_non_marked_component_is_not_transferred_from_client() {
    TestEnv::default().run(
        |_: &mut App, c: &mut App| c.world.spawn((SyncMark {}, MyNonSynched {})).id(),
        |s: &mut App, _: &mut App, _: Entity| {
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

//TODO: component sync first case:
//#[test]
fn test_marked_component_is_transferred_from_server() {
    TestEnv::default().run(
        |s: &mut App, _: &mut App| {
            s.sync_component::<MySynched>();
            s.world.spawn((SyncMark {}, MySynched {}));
            1
        },
        |s: &mut App, c: &mut App, entity_count: u32| {
            let mut count_check = 0;
            for e in c
                .world
                .query_filtered::<&SyncUp, With<MySynched>>()
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
