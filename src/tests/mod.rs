mod setup;
use crate::data::SyncComponent;

use super::*;
use bevy::reflect::ReflectFromReflect;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use setup::TestEnv;

#[test]
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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

#[derive(Component, Reflect, FromReflect, Default, PartialEq, Serialize, Deserialize, Debug)]
#[reflect(Component, FromReflect)]
pub struct MySynched {
    value: i32,
}

fn changes_of_my_synched(
    mut push: ResMut<SyncPusher>,
    q: Query<(Entity, &MySynched), Changed<MySynched>>,
) {
    for (e_id, component) in q.iter() {
        push.push(e_id, component.clone_value());
    }
}

#[test]
#[serial]
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
#[serial]
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

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server() {
    TestEnv::default().run(
        |s: &mut App, c: &mut App| {
            s.sync_component::<MySynched>();
            c.sync_component::<MySynched>();
            s.add_system(changes_of_my_synched);
            s.world.spawn((SyncMark {}, MySynched { value: 7 }));
            1
        },
        |s: &mut App, c: &mut App, entity_count: u32| {
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
