use std::{
    env,
    error::Error,
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{
    log::{Level, LogPlugin}, pbr::PbrPlugin, prelude::*, reflect::{DynamicTypePath, FromReflect, GetTypeRegistration, Reflect}, render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology}, state::app::StatesPlugin, MinimalPlugins
};
use bevy_renet::renet::RenetClient;
use bevy_sync::{ClientPlugin, ServerPlugin, SyncComponent, SyncPlugin};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Component)]
#[allow(dead_code)] // for some reason compiler thinks this is not used but it is
pub(crate) struct MyNonSynched;

#[derive(Component, Reflect, Default, PartialEq, Serialize, Deserialize, Debug)]
#[reflect(Component)]
pub(crate) struct MySynched {
    pub(crate) value: i32,
}

#[derive(Component, Reflect, Default, PartialEq, Serialize, Deserialize, Debug)]
#[reflect(Component)]
pub(crate) struct MySynched2 {
    pub(crate) value: i32,
}

#[derive(Debug)]
pub(crate) struct TestError(String);
impl Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for TestError {
    fn description(&self) -> &str {
        &self.0
    }
}

pub(crate) struct TestRun {
    pub(crate) port: u16,
    pub(crate) ip: IpAddr,
    pub(crate) startup_max_wait_updates: u32,
    pub(crate) updates_per_run: u32,
}

pub(crate) struct TestEnv {
    pub(crate) server: App,
    pub(crate) clients: Vec<App>,
}

impl TestEnv {
    #[allow(dead_code)]
    pub(crate) fn update(&mut self, count: u32) {
        for _ in 0..count {
            self.server.update();
            for capp in &mut self.clients {
                capp.update();
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn setup_registration<
        T: Component + TypePath + DynamicTypePath + Reflect + FromReflect + GetTypeRegistration,
    >(
        &mut self,
    ) {
        self.server.sync_component::<T>();
        for c in &mut self.clients {
            c.sync_component::<T>();
        }
    }
}

impl Default for TestRun {
    fn default() -> Self {
        Self {
            port: portpicker::pick_unused_port().expect("No ports free"),
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            startup_max_wait_updates: 20,
            updates_per_run: 20,
        }
    }
}

impl TestRun {
    #[allow(dead_code)]
    pub(crate) fn no_pre_setup(_: &mut TestEnv) {}

    #[allow(dead_code)]
    pub(crate) fn no_setup(_: &mut TestEnv) {}

    pub(crate) fn run<F0, F1, F2, T0, T1>(
        &self,
        client_count: u32,
        mut pre_connect: F0,
        mut post_connect: F1,
        mut assertion: F2,
    ) where
        F0: FnMut(&mut TestEnv) -> T0,
        F1: FnMut(&mut TestEnv) -> T1,
        F2: FnMut(&mut TestEnv, T0, T1),
    {
        let mut test_run = TestEnv {
            server: create_server().unwrap(),
            clients: vec![],
        };
        for _ in 0..client_count {
            test_run.clients.push(create_client().unwrap());
        }

        let x = pre_connect(&mut test_run);

        connect_envs(self, &mut test_run.server, &mut test_run.clients).unwrap();

        let y = post_connect(&mut test_run);

        let mut count = 0;
        while count < self.updates_per_run {
            test_run.server.update();
            for capp in &mut test_run.clients {
                capp.update();
            }
            count += 1;
        }
        assertion(&mut test_run, x, y);
    }
}

fn create_server() -> Result<App, Box<dyn Error>> {
    let mut sapp = App::new();
    add_plugins(&mut sapp);
    // Start a non synched entity only on server so the id is intentionally offseted between server and client
    sapp.world_mut().spawn(TransformBundle::default());
    Ok(sapp)
}

fn create_client() -> Result<App, Box<dyn Error>> {
    let mut capp = App::new();
    add_plugins(&mut capp);
    Ok(capp)
}

fn add_plugins(app: &mut App) {
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Shader>();
    app.init_asset::<Mesh>();
    app.init_asset::<Image>();
    app.add_plugins(PbrPlugin::default());
    if env::var("LOG").is_ok() {
        app.add_plugins(LogPlugin {
            filter: "error,bevy_sync=debug".to_string(),
            level: Level::DEBUG,
            ..default()
        });
    }

    app.add_plugins(SyncPlugin);
}

fn connect_envs(env: &TestRun, sapp: &mut App, capps: &mut [App]) -> Result<(), Box<dyn Error>> {
    sapp.add_plugins(ServerPlugin {
        ip: env.ip,
        port: env.port,
        web_port: portpicker::pick_unused_port().unwrap(),
        max_transfer: 100_000_000,
    });

    for capp in capps {
        capp.add_plugins(ClientPlugin {
            ip: env.ip,
            port: env.port,
            web_port: portpicker::pick_unused_port().unwrap(),
            max_transfer: 100_000_000,
        });

        wait_until_connected(sapp, capp, env.startup_max_wait_updates)?;
    }

    Ok(())
}

fn wait_until_connected(
    sapp: &mut App,
    capp: &mut App,
    updates: u32,
) -> Result<(), Box<dyn Error>> {
    sapp.update();
    capp.update();

    let mut count = 0;
    while count < updates {
        sapp.update();
        capp.update();
        if !capp.world().resource::<RenetClient>().is_disconnected() {
            return Ok(());
        }
        count += 1;
    }

    Err(TestError("Client did not connect.".into()).into())
}

#[allow(dead_code)]
pub(crate) fn spawn_new_material(app: &mut App) -> AssetId<StandardMaterial> {
    let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let id = Uuid::new_v4();
    let material = StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        ..Default::default()
    };
    let handle = Handle::<StandardMaterial>::Weak(id.into());
    materials.insert(id, material);

    app.world_mut().spawn(handle);

    id.into()
}

#[allow(dead_code)]
pub(crate) fn spawn_new_mesh(app: &mut App) -> AssetId<Mesh> {
    let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    let id = Uuid::new_v4();
    let mesh = sample_mesh();
    let handle = Handle::<Mesh>::Weak(id.into());
    meshes.insert(id, mesh);

    app.world_mut().spawn(handle);

    id.into()
}

#[allow(dead_code)]
pub(crate) fn spawn_new_image(app: &mut App) -> AssetId<Image> {
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let id = Uuid::new_v4();
    let mesh = sample_image();
    let handle = Handle::<Image>::Weak(id.into());
    images.insert(id, mesh);

    app.world_mut().spawn(handle);

    id.into()
}

#[allow(dead_code)]
pub(crate) fn spawn_new_material_nouuid(app: &mut App) -> Handle<StandardMaterial> {
    let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        ..Default::default()
    })
}

#[allow(dead_code)]
pub(crate) fn spawn_new_mesh_nouuid(app: &mut App) -> Handle<Mesh> {
    let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    meshes.add(sample_mesh())
}

#[allow(dead_code)]
pub(crate) fn sample_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0., 0., 0.], [1., 2., 1.], [2., 0., 0.]],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0., 1., 0., 0.]; 3]);
    mesh.insert_indices(Indices::U32(vec![0, 2, 1]));

    mesh
}

#[allow(dead_code)]
pub(crate) fn sample_image() -> Image {
    Image::default()
}
