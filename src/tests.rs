use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{net::UdpSocket, time::SystemTime};

use bevy_renet::renet::{
    ClientAuthentication, RenetClient, RenetConnectionConfig, RenetServer, ServerAuthentication,
    ServerConfig,
};
use bevy_renet::{RenetClientPlugin, RenetServerPlugin};
use serial_test::serial;

use super::proto::PROTOCOL_ID;
use super::*;

#[test]
#[serial]
fn test_connection_setup() {
    setup().unwrap();
}

#[test]
#[serial]
fn test_entity_copied() {
    let (mut server, mut client) = setup().unwrap();

    server.world.spawn(SyncUp::default());

    server.update();
    client.update();

    assert_eq!(server.world.entities().is_empty(), false);
    assert_eq!(client.world.entities().is_empty(), false);
}

fn setup() -> Result<(App, App), Box<dyn Error>> {
    let mut sapp = App::new();
    sapp.add_plugins(MinimalPlugins);
    sapp.add_plugin(RenetServerPlugin::default());
    sapp.insert_resource(create_server()?);

    sapp.add_plugin(ServerPlugin);

    let mut capp = App::new();
    capp.add_plugins(MinimalPlugins);
    capp.add_plugin(RenetClientPlugin::default());
    capp.insert_resource(create_client()?);

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

const SERVER_PORT: u16 = 4444;
const SERVER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

fn create_server() -> Result<RenetServer, Box<dyn Error>> {
    let server_addr = SocketAddr::new(SERVER_IP, SERVER_PORT);
    RenetServer::new(
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?,
        ServerConfig::new(64, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure),
        RenetConnectionConfig::default(),
        UdpSocket::bind(server_addr)?,
    )
    .map_err(From::from)
}

fn create_client() -> Result<RenetClient, Box<dyn Error>> {
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let ip = SERVER_IP;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        server_addr: SocketAddr::new(ip, SERVER_PORT),
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
