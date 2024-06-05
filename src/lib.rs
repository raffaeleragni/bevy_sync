/*!
# Bevy engine network synchronization
**state is in development**
*/

mod binreflect;
mod bundle_fix;
mod client;
mod lib_priv;
mod logging;
mod networking;
mod proto;
mod server;

pub use proto::PromoteToHostEvent;

pub mod prelude {
    pub use super::{
        ClientPlugin, ClientState, ServerPlugin, ServerState, SyncComponent, SyncDown, SyncExclude,
        SyncMark, SyncPlugin, SyncUp,
    };
}

use std::{marker::PhantomData, net::IpAddr};

use bevy::{prelude::*, reflect::*};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum ServerState {
    Connected,
    #[default]
    Disconnected,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum ClientState {
    ConnectedInitialSync,
    Connected,
    Connecting,
    #[default]
    Disconnected,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SyncMark;

#[derive(Component, Default)]
pub struct SyncExclude<T: Component> {
    marker: PhantomData<T>,
}

#[derive(Component)]
pub struct SyncDown {}

#[derive(Component)]
pub struct SyncUp {
    pub server_entity_id: Entity,
}

pub struct SyncPlugin;

pub struct ServerPlugin {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}

pub struct ClientPlugin {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}

pub trait SyncComponent {
    fn sync_component<
        T: Component + TypePath + DynamicTypePath + Reflect + FromReflect + GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self;
    fn sync_materials(&mut self, enable: bool);
    fn sync_meshes(&mut self, enable: bool);
}

#[derive(Resource)]
pub struct SyncConnectionParameters {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}
