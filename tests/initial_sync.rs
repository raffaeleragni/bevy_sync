mod assert;
mod setup;

use assert::{assets_has_mesh, material_has_color};
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_sync::{SyncComponent, SyncExclude, SyncMark};
use serial_test::serial;
use setup::{MySynched, TestEnv, TestRun};

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
            let mut materials = env.server.world.resource_mut::<Assets<StandardMaterial>>();
            let material = materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            });
            let id = material.id();
            env.server.world.spawn(material);

            let mut meshes = env.server.world.resource_mut::<Assets<Mesh>>();
            let mesh = meshes.add(sample_mesh());

            let m_id = mesh.id();
            env.server.world.spawn(mesh);

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
            assets_has_mesh(&mut env.clients[0], m_id);
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
            let mut materials = env.server.world.resource_mut::<Assets<StandardMaterial>>();
            let material = materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            });
            let id = material.id();
            env.server.world.spawn(material);

            let mut meshes = env.server.world.resource_mut::<Assets<Mesh>>();
            let mesh = meshes.add(sample_mesh());

            let m_id = mesh.id();
            env.server.world.spawn(mesh);

            (1, id, m_id)
        },
        TestRun::no_setup,
        |env: &mut TestEnv,
         (entity_count, id, m_id): (u32, AssetId<StandardMaterial>, AssetId<Mesh>),
         _| {
            for capp in &mut env.clients {
                assert::initial_sync_for_client_happened(&mut env.server, capp, entity_count);
                material_has_color(capp, id, Color::RED);
                assets_has_mesh(capp, m_id);
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

fn sample_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0., 0., 0.], [1., 2., 1.], [2., 0., 0.]],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0., 1., 0., 0.]; 3]);
    mesh.set_indices(Some(Indices::U32(vec![0, 2, 1])));

    mesh
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
