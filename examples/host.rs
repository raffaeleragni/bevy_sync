use std::{
    env,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{pbr::wireframe::Wireframe, prelude::*, render::primitives::Aabb};
use bevy_sync::{ServerPlugin, SyncComponent, SyncExclude, SyncMark, SyncPlugin};

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "bevy_sync=debug")
    }

    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 4000;
    let mut host = App::new();
    host.add_plugins(DefaultPlugins);
    //host.add_plugins(bevy_editor_pls::EditorPlugin::default());
    host.add_plugins(SyncPlugin);
    host.add_plugins(ServerPlugin { ip, port });

    host.sync_component::<Name>();
    host.sync_component::<Aabb>();
    host.sync_component::<Visibility>();
    host.sync_component::<Transform>();
    host.sync_component::<Wireframe>();
    host.sync_component::<PointLight>();
    host.sync_component::<Handle<StandardMaterial>>();
    host.sync_component::<Handle<Mesh>>();
    host.sync_materials(true);
    host.sync_meshes(true);

    host.add_systems(Startup, load_world);
    host.run();
}

fn load_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Plane::from_size(5.0).into()),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        },
        SyncMark,
        Name::new("Ground"),
        SyncExclude::<Name>::default(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Wireframe,
        SyncMark,
        Name::new("Cube"),
    ));
    commands.spawn((
        PointLightBundle {
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        },
        SyncMark,
        Name::new("Light"),
    ));
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
