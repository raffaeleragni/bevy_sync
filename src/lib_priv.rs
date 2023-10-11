use std::{any::TypeId, collections::VecDeque};

use bevy::{
    ecs::component::ComponentId,
    prelude::*,
    reflect::{DynamicTypePath, FromReflect, GetTypeRegistration, Reflect, ReflectFromReflect},
    utils::{HashMap, HashSet},
};
use bevy_renet::renet::ClientId;

use crate::{
    bundle_fix::BundleFixPlugin, client::ClientSyncPlugin, mesh_serde::bin_to_mesh, proto::AssId,
    proto_serde::bin_to_compo, server::ServerSyncPlugin, ClientPlugin, ServerPlugin, SyncComponent,
    SyncDown, SyncExclude, SyncMark, SyncPlugin, SyncUp,
};

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct ComponentChangeId {
    pub(crate) id: Entity,
    pub(crate) name: String,
}

pub(crate) struct ComponentChange {
    pub(crate) change_id: ComponentChangeId,
    pub(crate) data: Box<dyn Reflect>,
}

// Keeps mapping of server entity ids to client entity ids.
// Key: server entity id.
// Value: client entity id.
// For servers, the map contains same key & value.
#[derive(Resource, Default)]
pub(crate) struct SyncTrackerRes {
    pub(crate) server_to_client_entities: HashMap<Entity, Entity>,
    pub(crate) sync_components: HashSet<ComponentId>,
    pub(crate) exclude_components: HashMap<ComponentId, ComponentId>,
    pub(crate) changed_components: VecDeque<ComponentChange>,
    pushed_component_from_network: HashSet<ComponentChangeId>,
    pushed_handles_from_network: HashSet<AssId>,
    sync_materials: bool,
    sync_meshes: bool,
}

impl SyncTrackerRes {
    pub(crate) fn signal_component_changed(&mut self, id: Entity, data: Box<dyn Reflect>) {
        let name = data.reflect_type_path().into();
        let change_id = ComponentChangeId { id, name };
        if self.pushed_component_from_network.contains(&change_id) {
            self.pushed_component_from_network.remove(&change_id);
            return;
        }
        self.changed_components
            .push_back(ComponentChange { change_id, data });
    }

    pub(crate) fn skip_network_handle_change(&mut self, id: AssId) -> bool {
        if self.pushed_handles_from_network.contains(&id) {
            self.pushed_handles_from_network.remove(&id);
            return true;
        }
        false
    }

    pub(crate) fn apply_component_change_from_network(
        e_id: Entity,
        name: String,
        data: &[u8],
        world: &mut World,
    ) -> bool {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let component_data = bin_to_compo(data, &registry);
        let registration = registry.get_with_type_path(name.as_str()).unwrap();
        let reflect_component = registration.data::<ReflectComponent>().unwrap();
        let previous_value = reflect_component.reflect(world.entity(e_id));
        if SyncTrackerRes::needs_to_change(previous_value, &*component_data) {
            debug!(
                "Changed component from network: {}v{} - {}",
                e_id.index(),
                e_id.generation(),
                &name
            );
            world
                .resource_mut::<SyncTrackerRes>()
                .pushed_component_from_network
                .insert(ComponentChangeId { id: e_id, name });
            let entity = &mut world.entity_mut(e_id);
            reflect_component.apply_or_insert(entity, component_data.as_reflect());
            true
        } else {
            debug!(
                "Skipped component from network: {}v{} - {}",
                e_id.index(),
                e_id.generation(),
                name
            );
            false
        }
    }

    pub(crate) fn apply_material_change_from_network(
        id: AssId,
        material: &[u8],
        world: &mut World,
    ) {
        world
            .resource_mut::<SyncTrackerRes>()
            .pushed_handles_from_network
            .insert(id);
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let component_data = bin_to_compo(material, &registry);
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mat = *component_data.downcast::<StandardMaterial>().unwrap();
        materials.insert(id, mat);
    }

    pub(crate) fn apply_mesh_change_from_network(id: AssId, mesh: &[u8], world: &mut World) {
        world
            .resource_mut::<SyncTrackerRes>()
            .pushed_handles_from_network
            .insert(id);
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh = bin_to_mesh(mesh);
        meshes.insert(id, mesh);
    }

    fn needs_to_change(previous_value: Option<&dyn Reflect>, component_data: &dyn Reflect) -> bool {
        if previous_value.is_none() {
            return true;
        }
        !previous_value
            .unwrap()
            .reflect_partial_eq(component_data)
            .unwrap_or(true)
    }

    pub(crate) fn sync_materials_enabled(&self) -> bool {
        self.sync_materials
    }
}

#[allow(clippy::type_complexity)]
fn sync_detect_server<T: Component + Reflect>(
    mut push: ResMut<SyncTrackerRes>,
    q: Query<(Entity, &T), (With<SyncDown>, Without<SyncExclude<T>>, Changed<T>)>,
) {
    for (e_id, component) in q.iter() {
        push.signal_component_changed(e_id, component.clone_value());
    }
}

#[allow(clippy::type_complexity)]
fn sync_detect_client<T: Component + Reflect>(
    mut push: ResMut<SyncTrackerRes>,
    q: Query<(&SyncUp, &T), (With<SyncUp>, Without<SyncExclude<T>>, Changed<T>)>,
) {
    for (sup, component) in q.iter() {
        push.signal_component_changed(sup.server_entity_id, component.clone_value());
    }
}

impl SyncComponent for App {
    fn sync_component<
        T: Component + TypePath + DynamicTypePath + Reflect + FromReflect + GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self {
        self.register_type::<T>();
        self.register_type_data::<T, ReflectFromReflect>();
        let c_id = self.world.init_component::<T>();
        let c_exclude_id = self.world.init_component::<SyncExclude<T>>();
        let mut track = self.world.resource_mut::<SyncTrackerRes>();
        track.sync_components.insert(c_id);
        track.exclude_components.insert(c_id, c_exclude_id);
        self.add_systems(Update, sync_detect_server::<T>);
        self.add_systems(Update, sync_detect_client::<T>);

        setup_cascade_registrations::<T>(self);

        self
    }

    fn sync_materials(&mut self, enable: bool) {
        let mut tracker = self.world.resource_mut::<SyncTrackerRes>();
        tracker.sync_materials = enable;
    }

    fn sync_meshes(&mut self, enable: bool) {
        let mut tracker = self.world.resource_mut::<SyncTrackerRes>();
        tracker.sync_meshes = enable;
    }
}

fn setup_cascade_registrations<T: Component + Reflect + FromReflect + GetTypeRegistration>(
    app: &mut App,
) {
    if TypeId::of::<T>() == TypeId::of::<Handle<StandardMaterial>>() {
        app.register_type_data::<StandardMaterial, ReflectFromReflect>();
        app.register_type::<Color>();
        app.register_type::<Image>();
        app.register_type::<Handle<Image>>();
        app.register_type::<Option<Handle<Image>>>();
        app.register_type::<AlphaMode>();
        app.register_type::<ParallaxMappingMethod>();
    }

    if TypeId::of::<T>() == TypeId::of::<PointLight>() {
        app.register_type::<Color>();
    }
}

#[derive(Component)]
pub(crate) struct SyncClientGeneratedEntity {
    pub(crate) client_id: ClientId,
    pub(crate) client_entity_id: Entity,
}

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SyncMark>();
        app.init_resource::<SyncTrackerRes>();
        app.add_plugins(BundleFixPlugin);
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        crate::networking::setup_server(app, self.ip, self.port);
        app.add_plugins(ServerSyncPlugin);
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        crate::networking::setup_client(app, self.ip, self.port);
        app.add_plugins(ClientSyncPlugin);
    }
}

pub(crate) fn sync_material_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_materials
}

pub(crate) fn sync_mesh_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_meshes
}
