use std::{
    error::Error,
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{
    pbr::PbrPlugin,
    prelude::*,
    reflect::{FromReflect, GetTypeRegistration, Reflect},
    transform::TransformBundle,
    MinimalPlugins,
};
use bevy_renet::renet::RenetClient;
use bevy_sync::{ClientPlugin, ServerPlugin, SyncComponent, SyncPlugin};
use serde::{Deserialize, Serialize};

#[derive(Component)]
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
    pub(crate) fn setup_registration<T: Component + Reflect + FromReflect + GetTypeRegistration>(
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
            startup_max_wait_updates: 4,
            updates_per_run: 4,
        }
    }
}

impl TestRun {
    #[allow(dead_code)]
    pub(crate) fn no_pre_setup(_: &mut TestEnv) {}

    #[allow(dead_code)]
    pub(crate) fn no_setup(_: &mut TestEnv) {}

    pub(crate) fn run<F0, F1, F2, T1, T2>(
        &self,
        client_count: u32,
        mut pre_setup: F0,
        mut setup: F1,
        mut assertion: F2,
    ) where
        F0: FnMut(&mut TestEnv) -> T1,
        F1: FnMut(&mut TestEnv) -> T2,
        F2: FnMut(&mut TestEnv, T1, T2),
    {
        let mut test_run = TestEnv {
            server: create_server().unwrap(),
            clients: vec![],
        };
        for _ in 0..client_count {
            test_run.clients.push(create_client().unwrap());
        }

        let x = pre_setup(&mut test_run);

        connect_envs(self, &mut test_run.server, &mut test_run.clients).unwrap();

        let y = setup(&mut test_run);

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
    sapp.world.spawn(TransformBundle::default());
    Ok(sapp)
}

fn create_client() -> Result<App, Box<dyn Error>> {
    let mut capp = App::new();
    add_plugins(&mut capp);
    Ok(capp)
}

fn add_plugins(app: &mut App) {
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_asset::<Shader>().add_debug_asset::<Shader>();
    app.add_asset::<Mesh>().add_debug_asset::<Mesh>();
    app.add_plugins(PbrPlugin::default());

    app.add_plugins(SyncPlugin);
}

fn connect_envs(env: &TestRun, sapp: &mut App, capps: &mut Vec<App>) -> Result<(), Box<dyn Error>> {
    sapp.add_plugins(ServerPlugin {
        ip: env.ip,
        port: env.port,
    });

    for capp in capps {
        capp.add_plugins(ClientPlugin {
            ip: env.ip,
            port: env.port,
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
        if !capp.world.resource::<RenetClient>().is_disconnected() {
            return Ok(());
        }
        count += 1;
    }

    Err(TestError("Client did not connect.".into()).into())
}
