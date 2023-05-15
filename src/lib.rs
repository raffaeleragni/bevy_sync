mod proto;
mod receive;
mod send;

pub mod prelude {
    pub use super::{ClientPlugin, ServerPlugin, SyncMark};
}

use std::{
    error::Error,
    net::{IpAddr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::{
    renet::{
        ClientAuthentication, RenetClient, RenetConnectionConfig, RenetServer,
        ServerAuthentication, ServerConfig,
    },
    RenetClientPlugin, RenetServerPlugin,
};
use proto::PROTOCOL_ID;
use receive::{ClientReceivePlugin, ServerReceivePlugin};
use send::{ClientSendPlugin, ServerSendPlugin};

#[derive(Component)]
pub struct SyncDown {
    pub changed: bool,
}

impl Default for SyncDown {
    fn default() -> Self {
        Self { changed: true }
    }
}

#[derive(Component)]
pub struct SyncUp {
    pub changed: bool,
    pub server_entity_id: Entity,
}

#[derive(Component)]
pub struct SyncMark;
#[derive(Component)]
pub struct SyncClientGeneratedEntity {
    client_id: u64,
    client_entity_id: Entity,
}

pub struct ServerPlugin {
    pub port: u16,
    pub ip: IpAddr,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenetServerPlugin::default());
        app.insert_resource(create_server(self.ip, self.port).unwrap());
        app.add_plugin(ServerSendPlugin);
        app.add_plugin(ServerReceivePlugin);
    }
}

pub struct ClientPlugin {
    pub ip: IpAddr,
    pub port: u16,
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenetClientPlugin::default());
        app.insert_resource(create_client(self.ip, self.port).unwrap());
        app.add_plugin(ClientSendPlugin);
        app.add_plugin(ClientReceivePlugin);
    }
}

fn create_server(ip: IpAddr, port: u16) -> Result<RenetServer, Box<dyn Error>> {
    let socket = UdpSocket::bind((ip, port))?;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let server_addr = socket.local_addr()?;
    let config = ServerConfig::new(
        64,
        proto::PROTOCOL_ID,
        server_addr,
        ServerAuthentication::Unsecure,
    );
    RenetServer::new(now, config, RenetConnectionConfig::default(), socket).map_err(From::from)
}

fn create_client(ip: IpAddr, port: u16) -> Result<RenetClient, Box<dyn Error>> {
    let socket = UdpSocket::bind((ip, 0))?;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = now.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        server_addr: SocketAddr::new(ip, port),
        protocol_id: PROTOCOL_ID,
        user_data: None,
    };
    RenetClient::new(
        now,
        socket,
        RenetConnectionConfig::default(),
        authentication,
    )
    .map_err(From::from)
}

#[cfg(test)]
mod tests;
