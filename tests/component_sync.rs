mod assert;
mod setup;

use bevy::{pbr::CubemapVisibleEntities, prelude::*, render::primitives::CubemapFrusta};
use bevy_sync::{SyncExclude, SyncMark, SyncUp};
use serial_test::serial;
use setup::{MyNonSynched, MySynched, TestEnv, TestRun};

use crate::assert::{
    count_entities_with_component, count_entities_without_component, get_first_entity_component,
};

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.server.world.spawn((SyncMark {}, MyNonSynched {}));
            0
        },
        |env, _, _| {
            let count = count_entities_with_component::<MyNonSynched>(&mut env.clients[0]);
            assert_eq!(count, 0);
        },
    );
}

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.clients[0]
                .world
                .spawn((SyncMark {}, MyNonSynched {}))
                .id()
        },
        |env, _, _| {
            let count = count_entities_without_component::<MyNonSynched>(&mut env.clients[0]);
            assert_eq!(count, 0);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
        },
        |env, _, _| {
            let comp = get_first_entity_component::<MySynched>(&mut env.clients[0]).unwrap();
            assert_eq!(comp.value, 7);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server_then_changed() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(3);

            env.server
                .world
                .entity_mut(e_id)
                .insert(MySynched { value: 7 });
            env.update(3);

            env.server
                .world
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 3;
            env.update(3);

            0
        },
        |env, _, _| {
            let comp = get_first_entity_component::<MySynched>(&mut env.clients[0]).unwrap();
            assert_eq!(comp.value, 3);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.clients[0].world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.clients[0].world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            let server_e_id = e.get::<SyncUp>().unwrap().server_entity_id;
            server_e_id
        },
        |env, _, id: Entity| {
            let e = env.server.world.get_entity(id).unwrap();
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 7);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client_then_changed() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.clients[0].world.spawn(SyncMark {}).id();
            env.update(3);

            let mut e = env.clients[0].world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            env.update(3);

            env.clients[0]
                .world
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 3;
            env.update(3);

            let server_e_id = env.clients[0]
                .world
                .entity_mut(e_id)
                .get::<SyncUp>()
                .unwrap()
                .server_entity_id;
            server_e_id
        },
        |env: &mut TestEnv, _, id: Entity| {
            let e = env.server.world.get_entity(id).unwrap();
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 3);
        },
    );
}

#[test]
#[serial]
fn exclusion_marked_will_not_be_synced() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env
                .server
                .world
                .spawn((SyncMark {}, SyncExclude::<MySynched>::default()))
                .id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            0
        },
        |env, _, _| {
            let mut count_check = 0;
            for _ in env.clients[0]
                .world
                .query_filtered::<Entity, With<MySynched>>()
                .iter(&env.clients[0].world)
            {
                count_check += 1;
            }
            assert_eq!(count_check, 0);
        },
    );
}

#[test]
#[serial]
fn test_auto_spawn_for_global_transform() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<Transform>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(Transform::from_xyz(1.0, 2.0, 3.0));
        },
        |env, _, _| {
            let comp = get_first_entity_component::<Transform>(&mut env.clients[0]).unwrap();
            assert_eq!(comp.translation.x, 1.0);
            assert_eq!(comp.translation.y, 2.0);
            assert_eq!(comp.translation.z, 3.0);

            let _ = get_first_entity_component::<GlobalTransform>(&mut env.clients[0]).unwrap();
        },
    );
}

#[test]
#[serial]
fn test_auto_spawn_for_computed_visibility() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<Visibility>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(VisibilityBundle::default());
        },
        |env, _, _| {
            let _ = get_first_entity_component::<Visibility>(&mut env.clients[0]).unwrap();
            let _ = get_first_entity_component::<ViewVisibility>(&mut env.clients[0]).unwrap();
        },
    );
}

#[test]
#[serial]
fn test_auto_spawn_for_point_light() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<PointLight>();
            let e_id = env.server.world.spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world.entity_mut(e_id);
            e.insert(PointLightBundle::default());
        },
        |env, _, _| {
            let _ = get_first_entity_component::<PointLight>(&mut env.clients[0]).unwrap();
            let _ = get_first_entity_component::<CubemapFrusta>(&mut env.clients[0]).unwrap();
            let _ =
                get_first_entity_component::<CubemapVisibleEntities>(&mut env.clients[0]).unwrap();
        },
    );
}
