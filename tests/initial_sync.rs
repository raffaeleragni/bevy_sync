mod assert;
mod setup;

use assert::{assets_has_sample_mesh, material_has_color};
use bevy::prelude::*;
use bevy_sync::{SyncComponent, SyncExclude, SyncMark};
use serial_test::serial;
use setup::{
    spawn_new_material, spawn_new_material_nouuid, spawn_new_mesh, spawn_new_mesh_nouuid,
    MySynched, TestEnv, TestRun,
};

use crate::{assert::count_entities_with_component, setup::MySynched2};

#[test]
#[serial]
fn test_initial_world_sync_sent_from_server() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<MySynched>();
            env.setup_registration::<Handle<StandardMaterial>>();
            env.setup_registration::<Handle<Mesh>>();
            env.server.sync_materials(true);
            env.server.sync_meshes(true);
            let e_id = env.server.world.spawn(SyncMark {}).id();

            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });

            let id = spawn_new_material(&mut env.server);
            let m_id = spawn_new_mesh(&mut env.server);

            (1, id, m_id)
        },
        TestRun::no_setup,
        |env, (entity_count, id, m_id): (u32, AssetId<StandardMaterial>, AssetId<Mesh>), _| {
            assert::initial_sync_for_client_happened(
                &mut env.server,
                &mut env.clients[0],
                entity_count,
            );
            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_client(&mut env.clients[0]);

            material_has_color(&mut env.clients[0], id, Color::RED);
            assets_has_sample_mesh(&mut env.clients[0], m_id);
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
            env.setup_registration::<Handle<StandardMaterial>>();
            env.server.sync_materials(true);
            env.server.sync_meshes(true);
            let e_id = env.server.world.spawn(SyncMark {}).id();

            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });

            let id = spawn_new_material(&mut env.server);
            let m_id = spawn_new_mesh(&mut env.server);

            (1, id, m_id)
        },
        TestRun::no_setup,
        |env: &mut TestEnv,
         (entity_count, id, m_id): (u32, AssetId<StandardMaterial>, AssetId<Mesh>),
         _| {
            for capp in &mut env.clients {
                assert::initial_sync_for_client_happened(&mut env.server, capp, entity_count);
                material_has_color(capp, id, Color::RED);
                assets_has_sample_mesh(capp, m_id);
            }

            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_clients(&mut env.clients);
        },
    );
}

#[test]
#[serial]
fn test_initial_world_sync_not_transfer_excluded_components() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<MySynched>();
            env.setup_registration::<Handle<StandardMaterial>>();
            env.server.sync_materials(true);
            let e_id = env
                .server
                .world
                .spawn((SyncMark {}, SyncExclude::<MySynched>::default()))
                .id();

            let mut e = env.server.world.entity_mut(e_id);
            e.insert(MySynched { value: 7 });

            0
        },
        TestRun::no_setup,
        |env, _, _| {
            let count = count_entities_with_component::<MySynched>(&mut env.clients[0]);
            assert_eq!(count, 0);
        },
    );
}

#[test]
#[serial]
fn test_initial_with_parenting() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<MySynched>();
            env.setup_registration::<MySynched2>();
            let _ = env
                .server
                .world
                .spawn((SyncMark, MySynched { value: 7 }))
                .with_children(|parent| {
                    parent.spawn((SyncMark, MySynched2 { value: 8 }));
                })
                .id();

            0
        },
        TestRun::no_setup,
        |env, _, _| {
            env.update(20);

            let app = &mut env.clients[0];
            let entity_value = app
                .world
                .query_filtered::<&MySynched, Without<Parent>>()
                .iter(&app.world)
                .next();
            assert_eq!(entity_value.unwrap().value, 7);

            let child_value = app
                .world
                .query_filtered::<&MySynched2, With<Parent>>()
                .iter(&app.world)
                .next();
            assert_eq!(child_value.unwrap().value, 8);
        },
    );
}

#[test]
#[serial]
fn test_initial_world_sync_without_uuid() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<MySynched>();
            env.setup_registration::<Handle<StandardMaterial>>();
            env.setup_registration::<Handle<Mesh>>();
            env.server.sync_materials(true);
            env.server.sync_meshes(true);
            let e_id = env.server.world.spawn(SyncMark {}).id();

            let material_id = spawn_new_material_nouuid(&mut env.server);
            let mesh_id = spawn_new_mesh_nouuid(&mut env.server);

            let mut e = env.server.world.entity_mut(e_id);
            e.insert((MySynched { value: 7 }, material_id, mesh_id));

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
