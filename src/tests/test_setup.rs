use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::{prelude::App, MinimalPlugins};
use bevy_renet::{
    renet::{
        ClientAuthentication, RenetClient, RenetConnectionConfig, RenetServer,
        ServerAuthentication, ServerConfig,
    },
    RenetClientPlugin, RenetServerPlugin,
};

use crate::{ClientPlugin, ServerPlugin};

const PROTOCOL_ID: u64 = 1;

pub struct TestEnv {
    pub port: u16,
    pub ip: IpAddr,
    pub updates_per_run: u32,
}

impl Default for TestEnv {
    fn default() -> Self {
        Self {
            port: 4444,
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            updates_per_run: 1,
        }
    }
}

impl TestEnv {
    pub fn run<F1, F2>(&self, mut setup: F1, mut assertion: F2)
    where
        F1: FnMut(&mut App, &mut App) -> (),
        F2: FnMut(&mut App, &mut App) -> (),
    {
        let (mut server, mut client) = setup_env(self).unwrap();
        setup(&mut server, &mut client);
        for _ in [..self.updates_per_run] {
            server.update();
            client.update();
        }
        assertion(&mut server, &mut client);
    }
}

fn setup_env(env: &TestEnv) -> Result<(App, App), Box<dyn Error>> {
    let mut sapp = App::new();
    sapp.add_plugins(MinimalPlugins);
    sapp.add_plugin(RenetServerPlugin::default());
    sapp.insert_resource(create_server(env)?);

    sapp.add_plugin(ServerPlugin);

    let mut capp = App::new();
    capp.add_plugins(MinimalPlugins);
    capp.add_plugin(RenetClientPlugin::default());
    capp.insert_resource(create_client(env)?);

    capp.add_plugin(ClientPlugin);

    sapp.update();
    capp.update();

    loop {
        sapp.update();
        capp.update();
        if capp.world.resource::<RenetClient>().is_connected() {
            break;
        }
    }

    Ok((sapp, capp))
}

fn create_server(env: &TestEnv) -> Result<RenetServer, Box<dyn Error>> {
    let server_addr = SocketAddr::new(env.ip, env.port);
    RenetServer::new(
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?,
        ServerConfig::new(64, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure),
        RenetConnectionConfig::default(),
        UdpSocket::bind(server_addr)?,
    )
    .map_err(From::from)
}

fn create_client(env: &TestEnv) -> Result<RenetClient, Box<dyn Error>> {
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let ip = env.ip;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        server_addr: SocketAddr::new(ip, env.port),
        protocol_id: PROTOCOL_ID,
        user_data: None,
    };
    RenetClient::new(
        current_time,
        UdpSocket::bind((ip, 0))?,
        RenetConnectionConfig::default(),
        authentication,
    )
    .map_err(From::from)
}
