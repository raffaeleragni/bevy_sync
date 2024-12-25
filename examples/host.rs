use std::{
    env,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{prelude::*, render::primitives::Aabb};
use bevy_sync::prelude::*;
use uuid::Uuid;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "bevy_sync=debug")
    }

    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 4000;
    let web_port = 4000;
    let mut host = App::new();
    host.add_plugins(DefaultPlugins);
    // host.add_plugins(bevy_editor_pls::EditorPlugin::new());
    host.add_plugins(SyncPlugin);
    host.add_plugins(ServerPlugin {
        parameters: SyncConnectionParameters::Socket {
            ip,
            port,
            web_port,
            max_transfer: 100_000_000,
        },
    });

    host.sync_component::<Name>();
    host.sync_component::<Aabb>();
    host.sync_component::<Visibility>();
    host.sync_component::<Transform>();
    host.sync_component::<PointLight>();
    host.sync_component::<DirectionalLight>();
    host.sync_component::<SpotLight>();
    host.sync_component::<MeshMaterial3d<StandardMaterial>>();
    host.sync_component::<Mesh3d>();
    host.sync_materials(true);
    host.sync_meshes(true);
    host.sync_audios(true);

    host.add_systems(Startup, load_world);
    host.run();
}

trait AddByUuid<A: Asset> {
    fn addu(&mut self, asset: A) -> Handle<A>;
}
impl<A: Asset> AddByUuid<A> for Assets<A> {
    fn addu(&mut self, asset: A) -> Handle<A> {
        let id = AssetId::Uuid {
            uuid: Uuid::new_v4(),
        };
        self.insert(id, asset);
        Handle::<A>::Weak(id)
    }
}

fn load_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.addu(Plane3d::default().mesh().size(5.0, 5.0).into())),
        MeshMaterial3d(materials.addu(Color::srgb(0.3, 0.5, 0.3).into())),
        SyncMark,
        Name::new("Ground"),
        SyncExclude::<Name>::default(),
    ));
    commands.spawn((
        Mesh3d(meshes.addu(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
        MeshMaterial3d(materials.addu(Color::srgb(0.8, 0.7, 0.6).into())),
        Transform::from_xyz(0.0, 0.5, 0.0),
        SyncMark,
        Name::new("Cube"),
    ));
    commands.spawn((
        PointLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0),
        SyncMark,
        Name::new("Light Point"),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0),
        SyncMark,
        Name::new("Light Directional"),
    ));
    commands.spawn((
        SpotLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0),
        SyncMark,
        Name::new("Light Spot"),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
