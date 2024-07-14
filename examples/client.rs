use std::{
    env,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{pbr::wireframe::Wireframe, prelude::*, render::primitives::Aabb};
use bevy_sync::{ClientPlugin, SyncComponent, SyncPlugin};

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "bevy_sync=debug")
    }

    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 4000;
    let web_port = 4001;

    let mut client = App::new();
    client.add_plugins(DefaultPlugins);
    client.add_plugins(bevy_editor_pls::EditorPlugin::default());
    client.add_plugins(SyncPlugin);
    client.add_plugins(ClientPlugin {
        ip,
        port,
        web_port,
        max_transfer: 100_000_000,
    });

    client.sync_component::<Name>();
    client.sync_component::<Aabb>();
    client.sync_component::<Visibility>();
    client.sync_component::<Transform>();
    client.sync_component::<Wireframe>();
    client.sync_component::<PointLight>();
    client.sync_component::<Handle<StandardMaterial>>();
    client.sync_component::<Handle<Mesh>>();
    client.sync_component::<Handle<AudioSource>>();
    client.sync_materials(true);
    client.sync_meshes(true);
    client.sync_audios(true);

    client.add_systems(Startup, load_world);
    client.run();
}

fn load_world(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
