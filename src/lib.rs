mod proto;
mod receive;
mod send;

pub mod prelude {
    pub use super::{ClientPlugin, ServerPlugin, SyncUp};
}

use bevy::{app::PluginGroupBuilder, prelude::*};
use receive::ReceivePlugin;
use send::SendPlugin;

#[derive(Component)]
pub struct SyncUp {
    pub changed: bool,
}

impl Default for SyncUp {
    fn default() -> Self {
        Self { changed: true }
    }
}

#[derive(Component)]
pub struct SyncDown {
    pub server_entity: Entity,
}

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncPlugins);
    }
}

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncPlugins);
    }
}

pub struct SyncPlugins;

impl PluginGroup for SyncPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(SendPlugin)
            .add(ReceivePlugin)
    }
}

#[cfg(test)]
mod tests;
