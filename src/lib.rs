mod data;
mod proto;
mod receive_client;
mod receive_server;
mod send_from_client;
mod send_from_server;

use std::{
    collections::VecDeque,
    error::Error,
    net::{IpAddr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::{prelude::*, reflect::TypeRegistryInternal};
use bevy_renet::{
    renet::{
        ClientAuthentication, RenetClient, RenetConnectionConfig, RenetServer,
        ServerAuthentication, ServerConfig,
    },
    RenetClientPlugin, RenetServerPlugin,
};
use data::SyncDataPlugin;
use proto::PROTOCOL_ID;
use receive_client::ClientReceivePlugin;
use receive_server::ServerReceivePlugin;
use send_from_client::ClientSendPlugin;
use send_from_server::ServerSendPlugin;

pub mod prelude {
    pub use super::{ClientPlugin, ServerPlugin, SyncDown, SyncMark, SyncPusher, SyncUp};
}

#[derive(Component)]
pub struct SyncMark;

#[derive(Resource)]
pub struct SyncPusher {
    components: VecDeque<(Entity, Box<dyn Reflect>)>,
}

impl Default for SyncPusher {
    fn default() -> Self {
        Self {
            components: Default::default(),
        }
    }
}

impl SyncPusher {
    pub fn push(&mut self, e_id: Entity, component: Box<dyn Reflect>) {
        self.components.push_back((e_id, component));
    }
}

pub struct ServerPlugin {
    pub port: u16,
    pub ip: IpAddr,
}

pub struct ClientPlugin {
    pub ip: IpAddr,
    pub port: u16,
}

#[derive(Component)]
pub struct SyncDown {}

#[derive(Component)]
pub struct SyncUp {
    pub(crate) server_entity_id: Entity,
}

#[derive(Component)]
pub(crate) struct SyncClientGeneratedEntity {
    client_id: u64,
    client_entity_id: Entity,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncPusher>();
        app.add_plugin(RenetServerPlugin::default());
        app.insert_resource(create_server(self.ip, self.port).unwrap());
        app.add_plugin(SyncDataPlugin);
        app.add_plugin(ServerSendPlugin);
        app.add_plugin(ServerReceivePlugin);
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncPusher>();
        app.add_plugin(RenetClientPlugin::default());
        app.insert_resource(create_client(self.ip, self.port).unwrap());
        app.add_plugin(SyncDataPlugin);
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
