use std::{
    error::Error,
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{prelude::App, transform::TransformBundle, MinimalPlugins};
use bevy_renet::renet::RenetClient;

use crate::{ClientPlugin, ServerPlugin, SyncPlugin};

#[derive(Debug)]
pub struct TestError(String);
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

pub struct TestEnv {
    pub port: u16,
    pub ip: IpAddr,
    pub startup_max_wait_updates: u32,
    pub updates_per_run: u32,
}

impl Default for TestEnv {
    fn default() -> Self {
        Self {
            port: portpicker::pick_unused_port().expect("No ports free"),
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            startup_max_wait_updates: 1000,
            updates_per_run: 100,
        }
    }
}

impl TestEnv {
    pub fn run<F0, F1, F2, T1, T2>(&self, mut pre_setup: F0, mut setup: F1, mut assertion: F2)
    where
        F0: FnMut(&mut App, &mut App) -> T1,
        F1: FnMut(&mut App, &mut App) -> T2,
        F2: FnMut(&mut App, &mut App, T1, T2),
    {
        let (mut server, mut client) = create_env().unwrap();

        let x = pre_setup(&mut server, &mut client);

        connect_env(self, &mut server, &mut client).unwrap();

        let y = setup(&mut server, &mut client);

        let mut count = 0;
        while count < self.updates_per_run {
            server.update();
            client.update();
            count += 1;
        }
        assertion(&mut server, &mut client, x, y);
    }
}

fn create_env() -> Result<(App, App), Box<dyn Error>> {
    let mut sapp = App::new();
    sapp.add_plugins(MinimalPlugins);
    sapp.add_plugin(SyncPlugin);
    let mut capp = App::new();
    capp.add_plugins(MinimalPlugins);
    capp.add_plugin(SyncPlugin);
    // Start a non synched entity only on server so the id is intentionally offseted between server and client
    sapp.world.spawn(TransformBundle::default());
    Ok((sapp, capp))
}

fn connect_env(env: &TestEnv, sapp: &mut App, capp: &mut App) -> Result<(), Box<dyn Error>> {
    sapp.add_plugin(ServerPlugin {
        ip: env.ip,
        port: env.port,
    });

    capp.add_plugin(ClientPlugin {
        ip: env.ip,
        port: env.port,
    });

    wait_until_connected(sapp, capp, env.startup_max_wait_updates)?;

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
