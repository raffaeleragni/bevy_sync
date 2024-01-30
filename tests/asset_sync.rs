mod assert;
mod setup;

use assert::{assets_has_sample_image, assets_has_sample_mesh, material_has_color};
use bevy::prelude::*;
use bevy_sync::SyncComponent;
use serial_test::serial;
use setup::{spawn_new_image, spawn_new_material, spawn_new_mesh, TestRun};

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

#[serial]
#[test]
fn test_image_transferred_from_server() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<StandardMaterial>>();
            env.setup_registration::<Handle<Image>>();
            env.server.sync_materials(true);
            env.clients[0].sync_materials(true);
        },
        |env| {
            let app = &mut env.server;
            spawn_new_image(app)
        },
        |env, _, id| {
            assets_has_sample_image(&mut env.clients[0], id);
        },
    );
}

#[serial]
#[test]
fn test_images_transferred_from_client() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<Handle<StandardMaterial>>();
            env.setup_registration::<Handle<Image>>();
            env.server.sync_materials(true);
            env.clients[0].sync_materials(true);
        },
        |env| {
            let app = &mut env.clients[0];
            spawn_new_image(app)
        },
        |env, _, id| {
            assets_has_sample_image(&mut env.server, id);
        },
    );
}
