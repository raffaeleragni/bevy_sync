mod assert;
mod setup;

use assert::{assets_has_sample_mesh, material_has_color};
use bevy::prelude::*;
use bevy_sync::SyncComponent;
use serial_test::serial;
use setup::{spawn_new_material, spawn_new_mesh, TestRun, load_cube};

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
            let app = &mut env.server;
            spawn_new_material(app)
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
            spawn_new_material(app)
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
            spawn_new_material(app)
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
            spawn_new_mesh(app)
        },
        |env, _, id| {
            assets_has_sample_mesh(&mut env.clients[0], id);
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
            spawn_new_mesh(app)
        },
        |env, _, id| {
            assets_has_sample_mesh(&mut env.server, id);
        },
    );
}

#[test]
#[serial]
fn test_with_asset_loader() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<Mesh>>();
            env.setup_registration::<Handle<StandardMaterial>>();
            for app in [&mut env.server, &mut env.clients[0]] {
                app.sync_meshes(true);
                app.sync_materials(true);
            }
        },
        |env| {
            let app = &mut env.server;
            load_cube(app)
        },
        |env, _, (mesh_id, material_id)| {
            println!("{:?} {:?}", mesh_id, material_id);
            assets_has_sample_mesh(&mut env.clients[0], mesh_id);
            material_has_color(&mut env.clients[0], material_id, Color::RED);
        },
    );
}

