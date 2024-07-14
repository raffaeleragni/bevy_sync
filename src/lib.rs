//! bevy_sync
//!
//! Plugin for synchronizing entities and components between server and its clients.
//!
//! The synchronization is started with the setup:
//!
//! ```
//! use bevy::prelude::*;
//! use bevy_sync::prelude::*;
//! use std::{env, net::{IpAddr, Ipv4Addr}};
//! use bevy_sync::SyncComponent;
//!
//! let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
//! let mut app = App::new();
//! app.add_plugins(DefaultPlugins);
//! app.add_plugins(SyncPlugin);
//! app.add_plugins(ServerPlugin {
//!     ip,                          // binding network address
//!     port: 4000,                  // port for the main sync protocol (udp)
//!     web_port: 4001,              // port for http assets transfer
//!     max_transfer: 100_000_000,   // maximum size allowed for asset transfer
//! });
//!
//! // Specify which components and assets to sync
//!
//! app.sync_component::<Transform>();
//! app.sync_component::<Handle<StandardMaterial>>();
//! app.sync_component::<Handle<Mesh>>();
//! app.sync_component::<Handle<AudioSource>>();
//! app.sync_materials(true);
//! app.sync_meshes(true);
//! app.sync_audios(true);
//! ```
//!

/// Use this event to promote one of the clients as host
pub use proto::PromoteToHostEvent;
pub use uuid::Uuid;
pub mod prelude {
    pub use super::{
        ClientPlugin, ClientState, ServerPlugin, ServerState, SyncComponent, SyncEntity,
        SyncExclude, SyncMark, SyncPlugin,
    };
}

mod binreflect;
mod bundle_fix;
mod client;
mod lib_priv;
mod logging;
mod networking;
mod proto;
mod server;

use bevy::{prelude::*, reflect::*};
use std::{marker::PhantomData, net::IpAddr};

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

/// Use this component to mark which entities to be synched.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SyncMark;

/// Use this component to mark which compoonent in the entity to exclude from sync.
#[derive(Component, Default)]
pub struct SyncExclude<T: Component> {
    marker: PhantomData<T>,
}

/// Keeps track of the entity uuid used by bevy_sync across clients
#[derive(Component)]
pub struct SyncEntity {
    pub uuid: Uuid,
}

/// Main bevy_syng plugin to setup for sync
pub struct SyncPlugin;

/// Plugin used for hosting mode
pub struct ServerPlugin {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}

/// Plugin used for joining a host
pub struct ClientPlugin {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}

/// Use this trait extension to configure sync details for your app
pub trait SyncComponent {
    fn sync_component<
        T: Component + TypePath + DynamicTypePath + Reflect + FromReflect + GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self;
    fn sync_materials(&mut self, enable: bool);
    fn sync_meshes(&mut self, enable: bool);
    fn sync_audios(&mut self, enable: bool);
}

/// You can use this resource to access which connection is currently being used and by which
/// options
#[derive(Resource)]
pub struct SyncConnectionParameters {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}
