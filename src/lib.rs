/*!
# Bevy engine network synchronization
**state is in development**

Wire up server and clients to synchronize their entities, components and assets,
mainly intended for collaborative editing combined with `bevy_editor_pls` crate.

Networking is through UDP (renet standard) and changes are sent through a ordered+reliable channel
with idempotent messages.

Uses:
  - `bevy`
  - `bevy_renet`
  - `serde` & `bincode`

## Example

```rust
use std::net::Ipv4Addr;
use bevy::{prelude::*, MinimalPlugins,};
use bevy_sync::{ServerPlugin, SyncComponent, SyncMark, SyncPlugin};

let mut app = App::new();
app.add_plugins(MinimalPlugins);

// Either one of these two, if being server or client
app.add_plugins(ServerPlugin { ip: Ipv4Addr::LOCALHOST.into(), port: 5555 });
//app.add_plugin(ClientPlugin { ip: Ipv4Addr::LOCALHOST.into(), port: 5555 });

// Setup sync mechanics and which components will be synced
app.add_plugins(SyncPlugin);
app.sync_component::<Transform>();

// Mark entity for sync with SyncMark component
app.world.spawn(Transform::default()).insert(SyncMark {});
```

*/

mod bundle_fix;
mod client;
mod lib_priv;
mod mesh_serde;
mod networking;
mod proto;
mod proto_serde;
mod server;

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
pub struct SyncExclude<T>
where
    T: Component,
{
    marker: PhantomData<T>,
}

pub struct SyncPlugin;

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
    pub server_entity_id: Entity,
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
