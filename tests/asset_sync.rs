mod assert;
mod setup;

use assert::{assets_has_mesh, material_has_color};
use bevy::{
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
};
use bevy_sync::SyncComponent;
use serial_test::serial;
use setup::TestRun;

#[test]
#[serial]
fn sync_material_from_server() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<StandardMaterial>>();
            env.server.sync_materials(true);
        },
        |env| {
            let s = &mut env.server;
            let mut materials = s.world.resource_mut::<Assets<StandardMaterial>>();
            let material = materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            });

            let id = material.id();
            s.world.spawn(material);

            id
        },
        |env, _, id| {
            material_has_color(&mut env.clients[0], id, Color::RED);
        },
    );
}

#[test]
#[serial]
fn sync_material_from_client() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<StandardMaterial>>();
            env.server.sync_materials(true);
            env.clients[0].sync_materials(true);
        },
        |env| {
            let app = &mut env.clients[0];
            let mut materials = app.world.resource_mut::<Assets<StandardMaterial>>();
            let material = materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            });

            let id = material.id();
            app.world.spawn(material);

            id
        },
        |env, _, id| {
            material_has_color(&mut env.clients[0], id, Color::RED);
        },
    );
}

#[test]
#[serial]
fn sync_material_from_client_to_client_across_server() {
    TestRun::default().run(
        2,
        |env| {
            env.setup_registration::<Handle<StandardMaterial>>();
            env.server.sync_materials(true);
            env.clients[0].sync_materials(true);
        },
        |env| {
            let app = &mut env.clients[0];
            let mut materials = app.world.resource_mut::<Assets<StandardMaterial>>();
            let material = materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            });

            let id = material.id();
            app.world.spawn(material);

            id
        },
        |env, _, id| {
            material_has_color(&mut env.clients[0], id, Color::RED);
        },
    );
}

#[serial]
#[test]
fn test_mesh_transferred_from_server() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<Mesh>>();
            env.server.sync_meshes(true);
            env.clients[0].sync_meshes(true);
        },
        |env| {
            let app = &mut env.server;
            let mut meshes = app.world.resource_mut::<Assets<Mesh>>();
            let mesh = meshes.add(sample_mesh());

            let id = mesh.id();
            app.world.spawn(mesh);

            id
        },
        |env, _, id| {
            assets_has_mesh(&mut env.clients[0], id);
        },
    );
}

#[serial]
#[test]
fn test_mesh_transferred_from_client() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<Mesh>>();
            env.server.sync_meshes(true);
            env.clients[0].sync_meshes(true);
        },
        |env| {
            let app = &mut env.clients[0];
            let mut meshes = app.world.resource_mut::<Assets<Mesh>>();
            let mesh = meshes.add(sample_mesh());

            let id = mesh.id();
            app.world.spawn(mesh);

            id
        },
        |env, _, id| {
            assets_has_mesh(&mut env.server, id);
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
    mesh.set_indices(Some(mesh::Indices::U32(vec![0, 2, 1])));

    mesh
}
