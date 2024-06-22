use std::{any::TypeId, collections::VecDeque};

use bevy::{
    ecs::component::ComponentId,
    pbr::OpaqueRendererMethod,
    prelude::*,
    reflect::{DynamicTypePath, FromReflect, GetTypeRegistration, Reflect, ReflectFromReflect},
    utils::{HashMap, HashSet},
};
use uuid::Uuid;

use crate::{
    binreflect::bin_to_reflect, bundle_fix::BundleFixPlugin, client::ClientSyncPlugin,
    proto::AssId, server::ServerSyncPlugin, ClientPlugin, ClientState, PromoteToHostEvent,
    ServerPlugin, ServerState, SyncComponent, SyncConnectionParameters, SyncEntity, SyncExclude,
    SyncMark, SyncPlugin,
};

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct ComponentChangeId {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

pub(crate) struct ComponentChange {
    pub(crate) change_id: ComponentChangeId,
    pub(crate) data: Box<dyn Reflect>,
}

#[derive(Event)]
pub(crate) struct PromotedToServer;

#[derive(Event)]
pub(crate) struct PromotedToClient;

#[derive(Resource, Default)]
pub(crate) struct SyncTrackerRes {
    /// Mapping of entity ids between server and clients. key: server, value: client
    pub(crate) uuid_to_entity: HashMap<Uuid, Entity>,
    pub(crate) entity_to_uuid: HashMap<Entity, Uuid>,

    pub(crate) registered_componets_for_sync: HashSet<ComponentId>,
    /// Tracks SyncExcludes for component T. key: component id of T, value: component id of SyncExcdlude<T>
    pub(crate) sync_exclude_cid_of_component_cid: HashMap<ComponentId, ComponentId>,
    /// Queue of component changes to be sent over network
    pub(crate) changed_components_to_send: VecDeque<ComponentChange>,
    /// Pushed references (component and handle) that came from network and were applied in world,
    /// so that in the next detect step they will be skipped and avoid ensless loop.
    pub(crate) pushed_component_from_network: HashSet<ComponentChangeId>,
    pub(crate) pushed_handles_from_network: HashSet<AssId>,

    pub(crate) sync_materials: bool,
    pub(crate) sync_meshes: bool,
}

pub(crate) fn sync_material_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_materials
}

pub(crate) fn sync_mesh_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_meshes
}

impl SyncTrackerRes {
    pub(crate) fn signal_component_changed(&mut self, id: Uuid, data: Box<dyn Reflect>) {
        let name = data.get_represented_type_info().unwrap().type_path().into();
        let change_id = ComponentChangeId { id, name };
        if self.pushed_component_from_network.contains(&change_id) {
            self.pushed_component_from_network.remove(&change_id);
            return;
        }
        self.changed_components_to_send
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
        let component_data = bin_to_reflect(data, &registry);
        let registration = registry.get_with_type_path(name.as_str()).unwrap();
        let reflect_component = registration.data::<ReflectComponent>().unwrap();
        let Some(sync_entity) = world.entity(e_id).get::<SyncEntity>() else {
            return false;
        };
        let uuid = sync_entity.uuid;
        let previous_value = reflect_component.reflect(world.entity(e_id));
        let change_id = ComponentChangeId {
            id: uuid,
            name: name.clone(),
        };
        if world
            .resource::<SyncTrackerRes>()
            .pushed_component_from_network
            .get(&change_id)
            .is_some()
        {
            debug!(
                "Skipped component from network: {}v{} - {}",
                e_id.index(),
                e_id.generation(),
                name
            );
            return false;
        }
        if is_value_different(previous_value, &*component_data) {
            world
                .resource_mut::<SyncTrackerRes>()
                .pushed_component_from_network
                .insert(change_id);
            let entity = &mut world.entity_mut(e_id);
            reflect_component.apply_or_insert(entity, component_data.as_reflect(), &registry);
            debug!(
                "Applied component from network: {}v{} - {}",
                e_id.index(),
                e_id.generation(),
                name
            );
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
        let component_data = bin_to_reflect(material, &registry);
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mat = *component_data.downcast::<StandardMaterial>().unwrap();
        materials.insert(id, mat);
    }
}

fn is_value_different(previous_value: Option<&dyn Reflect>, component_data: &dyn Reflect) -> bool {
    if previous_value.is_none() {
        return true;
    }
    !previous_value
        .unwrap()
        .reflect_partial_eq(component_data)
        .unwrap_or(true)
}

#[allow(clippy::type_complexity)]
fn sync_detect<T: Component + Reflect>(
    mut push: ResMut<SyncTrackerRes>,
    q: Query<(&SyncEntity, &T), (With<SyncEntity>, Without<SyncExclude<T>>, Changed<T>)>,
) {
    for (sup, component) in q.iter() {
        push.signal_component_changed(sup.uuid, component.clone_value());
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
        track.registered_componets_for_sync.insert(c_id);
        track
            .sync_exclude_cid_of_component_cid
            .insert(c_id, c_exclude_id);
        self.add_systems(Update, sync_detect::<T>);

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
        app.register_type::<OpaqueRendererMethod>();
    }

    if TypeId::of::<T>() == TypeId::of::<PointLight>() {
        app.register_type::<Color>();
    }
}

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SyncMark>();
        app.init_resource::<SyncTrackerRes>();
        app.add_plugins(BundleFixPlugin);
        app.add_plugins(ServerSyncPlugin);
        app.add_plugins(ClientSyncPlugin);
        app.init_state::<ServerState>();
        app.init_state::<ClientState>();
        app.add_event::<PromotedToServer>();
        app.add_event::<PromotedToClient>();
        app.add_event::<PromoteToHostEvent>();
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SyncConnectionParameters {
            ip: self.ip,
            port: self.port,
            web_port: self.web_port,
            max_transfer: self.max_transfer,
        });
        crate::networking::setup_server(app, self.ip, self.port, self.web_port, self.max_transfer);
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SyncConnectionParameters {
            ip: self.ip,
            port: self.port,
            web_port: self.web_port,
            max_transfer: self.max_transfer,
        });
        crate::networking::setup_client(app, self.ip, self.port, self.web_port, self.max_transfer);
    }
}
