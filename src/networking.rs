use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::{
    renet::{
        transport::{
            ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport,
            ServerAuthentication, ServerConfig,
        },
        ConnectionConfig, RenetClient, RenetServer,
    },
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};

const PROTOCOL_ID: u64 = 1;

pub(crate) fn setup_server(app: &mut App, ip: IpAddr, port: u16) {
    app.add_plugins(RenetServerPlugin);
    app.insert_resource(RenetServer::new(ConnectionConfig::default()));
    app.add_plugins(NetcodeServerPlugin);
    app.insert_resource(create_server(ip, port));
}

pub(crate) fn setup_client(app: &mut App, ip: IpAddr, port: u16) {
    app.add_plugins(RenetClientPlugin);
    app.insert_resource(RenetClient::new(ConnectionConfig::default()));
    app.add_plugins(NetcodeClientPlugin);
    app.insert_resource(create_client(ip, port));
}

fn create_server(ip: IpAddr, port: u16) -> NetcodeServerTransport {
    let socket = UdpSocket::bind((ip, port)).unwrap();
    let server_addr = socket.local_addr().unwrap();
    const MAX_CLIENTS: usize = 64;
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server_config = ServerConfig {
        current_time,
        max_clients: MAX_CLIENTS,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![server_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    NetcodeServerTransport::new(server_config, socket).unwrap()
}

pub(crate) fn create_client(ip: IpAddr, port: u16) -> NetcodeClientTransport {
    let socket = UdpSocket::bind((ip, 0)).unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = now.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        server_addr: SocketAddr::new(ip, port),
        protocol_id: PROTOCOL_ID,
        user_data: None,
    };
    NetcodeClientTransport::new(now, authentication, socket).unwrap()
}
