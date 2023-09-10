mod assert;
mod setup;

use assert::material_has_color;
use bevy::prelude::*;
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
