mod assert;
mod setup;

use bevy::{
    asset::HandleId,
    prelude::{Assets, Color, Handle, StandardMaterial},
};
use bevy_sync::{SyncComponent, SyncMark};
use serial_test::serial;
use setup::{MySynched, TestEnv, TestRun};

#[test]
#[serial]
fn test_initial_world_sync_sent_from_server() {
    TestRun::default().run(
        1,
        |env| {
            env.setup_registration::<MySynched>();
            env.setup_registration::<Handle<StandardMaterial>>();
            env.server.sync_materials(true);
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

            (1, id)
        },
        TestRun::no_setup,
        |env, (entity_count, id): (u32, HandleId), _| {
            assert::initial_sync_for_client_happened(
                &mut env.server,
                &mut env.clients[0],
                entity_count,
            );
            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_client(&mut env.clients[0]);

            let materials = env.clients[0]
                .world
                .resource_mut::<Assets<StandardMaterial>>();
            let handle = materials.get_handle(id);
            let material = materials.get(&handle).unwrap();
            assert_eq!(material.base_color, Color::RED);
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

            (1, id)
        },
        TestRun::no_setup,
        |env: &mut TestEnv, (entity_count, id): (u32, HandleId), _| {
            for capp in &mut env.clients {
                assert::initial_sync_for_client_happened(&mut env.server, capp, entity_count);
                let materials = capp.world.resource_mut::<Assets<StandardMaterial>>();
                let handle = materials.get_handle(id);
                let material = materials.get(&handle).unwrap();
                assert_eq!(material.base_color, Color::RED);
            }

            assert::no_messages_left_for_server(&mut env.server);
            assert::no_messages_left_for_clients(&mut env.clients);
        },
    );
}
