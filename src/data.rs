use bevy::{
    ecs::component::ComponentId,
    prelude::{App, Component, Entity, Plugin, Resource},
    utils::{HashMap, HashSet},
};

// Keeps mapping of server entity ids to client entity ids.
// Key: server entity id.
// Value: client entity id.
// For servers, the map contains same key & value.
#[derive(Resource, Default)]
pub(crate) struct SyncTrackerRes {
    pub(crate) server_to_client_entities: HashMap<Entity, Entity>,
    pub(crate) sync_components: HashSet<ComponentId>,
}

pub(crate) struct SyncDataPlugin;

impl Plugin for SyncDataPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SyncTrackerRes>();
    }
}

pub trait SyncComponent {
    fn sync_component<T: Component>(&mut self) -> &mut Self;
}

impl SyncComponent for App {
    fn sync_component<T: Component>(&mut self) -> &mut Self {
        let c_id = self.world.init_component::<T>();
        let mut data = self.world.resource_mut::<SyncTrackerRes>();
        data.sync_components.insert(c_id);
        self
    }
}
