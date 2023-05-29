use bevy::{
    ecs::{archetype::Archetype, component::ComponentId},
    prelude::{App, Component, Entity, Plugin, Resource},
    reflect::{GetTypeRegistration, Reflect},
    utils::{HashMap, HashSet},
};

use crate::{ClientPlugin, ServerPlugin};

// Keeps mapping of server entity ids to client entity ids.
// Key: server entity id.
// Value: client entity id.
// For servers, the map contains same key & value.
#[derive(Resource, Default)]
pub(crate) struct SyncTrackerRes {
    pub(crate) server_to_client_entities: HashMap<Entity, Entity>,
    pub(crate) sync_components: HashSet<ComponentId>,
}

impl SyncTrackerRes {
    pub(crate) fn is_synched_archetype(&self, archetype: &Archetype) -> bool {
        for c_id in archetype.components().into_iter() {
            if self.is_synched_component(&c_id) {
                return true;
            }
        }
        false
    }
    pub(crate) fn is_synched_component(&self, c_id: &ComponentId) -> bool {
        self.sync_components.contains(c_id)
    }
}

pub(crate) struct SyncDataPlugin;

impl Plugin for SyncDataPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SyncTrackerRes>();
    }
}

pub trait SyncComponent {
    fn sync_component<T: Component + Reflect + GetTypeRegistration>(&mut self) -> &mut Self;
}

impl SyncComponent for App {
    fn sync_component<T: Component + Reflect + GetTypeRegistration>(&mut self) -> &mut Self {
        self.register_type::<T>();
        let c_id = self.world.init_component::<T>();
        let mut data = self.world.resource_mut::<SyncTrackerRes>();
        data.sync_components.insert(c_id);
        self.add_system(ServerPlugin::sync_detect::<T>);
        self.add_system(ClientPlugin::sync_detect::<T>);
        self
    }
}
