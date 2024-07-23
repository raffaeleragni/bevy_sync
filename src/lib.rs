//! bevy_sync
//!
//! Plugin for synchronizing entities and components between server and its clients.

/// Use this event to promote one of the clients as host
pub use proto::PromoteToHostEvent;
pub use uuid::Uuid;
pub mod prelude {
    pub use super::{
        proto::PromoteToHostEvent, ClientPlugin, ClientState, ServerPlugin, ServerState,
        SyncComponent, SyncConnectionParameters, SyncEntity, SyncExclude, SyncMark, SyncPlugin,
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

/// Use this component to mark which entities to be synched.
/// This component will be replaced with SyncEntity once the system engages on it.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SyncMark;

/// Keeps track of the entity uuid used by bevy_sync across clients.
/// This is automatically created on entities marked with SyncMark.
/// You don't need to add this one, but it's still available to distinguish later on
/// which entity is synched and which unique id is being used for it.
#[derive(Component)]
pub struct SyncEntity {
    pub uuid: Uuid,
}

/// Use this component to mark which component in the entity to exclude from sync.
/// This will skip synchronization only for the specific entity that is marked by this
/// component, and onlt for the component T inside that entity.
/// To skip more components into an entity, add more variations of this for more types.
#[derive(Component, Default)]
pub struct SyncExclude<T: Component> {
    marker: PhantomData<T>,
}

/// Specify networking options to create a session. This will also be available as a resource.
#[derive(Resource, Clone)]
pub struct SyncConnectionParameters {
    pub ip: IpAddr,
    pub port: u16,
    pub web_port: u16,
    pub max_transfer: usize,
}

/// Main bevy_sync plugin to setup for sync
/// Add this to the bevy app minimally, then either ServerPlugin or ClientPlugin.
pub struct SyncPlugin;

/// Plugin used for hosting mode
pub struct ServerPlugin {
    pub parameters: SyncConnectionParameters,
}

/// Plugin used for joining a host
pub struct ClientPlugin {
    pub parameters: SyncConnectionParameters,
}

/// Published state for server connectivity.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum ServerState {
    Connected,
    #[default]
    Disconnected,
}

/// Published state for client connectivity.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum ClientState {
    ConnectedInitialSync,
    Connected,
    Connecting,
    #[default]
    Disconnected,
}

/// Use this trait extension to configure sync details for your app.
/// Every component that needs to be synched must be called with sync_component.
/// To enable assets synching, use the other sync_* methods.
/// By default nothing is being synched, so you'll need to additively call all these.
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
