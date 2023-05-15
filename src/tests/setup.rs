use std::{
    error::Error,
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
};

use bevy::{prelude::App, transform::TransformBundle, MinimalPlugins};
use bevy_renet::renet::RenetClient;

use crate::{ClientPlugin, ServerPlugin};

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
    pub fn run<F1, F2, T>(&self, mut setup: F1, mut assertion: F2)
    where
        F1: FnMut(&mut App, &mut App) -> T,
        F2: FnMut(&mut App, &mut App, T),
    {
        let (mut server, mut client) = setup_env(self).unwrap();
        let x = setup(&mut server, &mut client);
        let mut count = 0;
        while count < self.updates_per_run {
            server.update();
            client.update();
            count += 1;
        }
        assertion(&mut server, &mut client, x);
    }
}

fn setup_env(env: &TestEnv) -> Result<(App, App), Box<dyn Error>> {
    let mut sapp = App::new();
    sapp.add_plugins(MinimalPlugins);
    let mut capp = App::new();
    capp.add_plugins(MinimalPlugins);

    // Start an entity only on server so the IDs intentionally offset between server and client
    sapp.world.spawn(TransformBundle::default());
    sapp.add_plugin(ServerPlugin {
        ip: env.ip,
        port: env.port,
    });

    capp.add_plugin(ClientPlugin {
        ip: env.ip,
        port: env.port,
    });

    wait_until_connected(&mut sapp, &mut capp, env.startup_max_wait_updates)?;

    Ok((sapp, capp))
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
        if capp.world.resource::<RenetClient>().is_connected() {
            return Ok(());
        }
        count += 1;
    }

    Err(TestError("Client did not connect.".into()).into())
}
