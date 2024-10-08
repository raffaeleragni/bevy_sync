use std::{any::TypeId, collections::VecDeque};

use bevy::{
    ecs::component::ComponentId,
    pbr::OpaqueRendererMethod,
    prelude::*,
    reflect::{DynamicTypePath, FromReflect, GetTypeRegistration, Reflect, ReflectFromReflect},
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    utils::{HashMap, HashSet},
};
use uuid::Uuid;

use crate::{
    binreflect::bin_to_reflect, bundle_fix::BundleFixPlugin, client::ClientSyncPlugin, proto::AssId, server::ServerSyncPlugin, ClientPlugin, ClientState, InitialSyncFinished, PromoteToHostEvent, ServerPlugin, ServerState, SyncComponent, SyncEntity, SyncExclude, SyncMark, SyncPlugin
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
    pub(crate) sync_audios: bool,

    pub(crate) host_promotion_in_progress: bool,
}

pub(crate) fn sync_material_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_materials
}

pub(crate) fn sync_mesh_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_meshes
}

pub(crate) fn sync_audio_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_audios
}

impl SyncTrackerRes {
    pub(crate) fn signal_component_changed(&mut self, id: Uuid, data: Box<dyn Reflect>) {
        let name = data.get_represented_type_info().unwrap().type_path().into();
        let change_id = ComponentChangeId { id, name };
        if self.pushed_component_from_network.contains(&change_id) {
            debug!(
                "Debouncing changed component, was already pushed. {:?},{:?}",
                change_id.id, change_id.name
            );
            self.pushed_component_from_network.remove(&change_id);
            return;
        }
        self.changed_components_to_send
            .push_back(ComponentChange { change_id, data });
    }

    pub(crate) fn skip_network_handle_change(&mut self, id: AssId) -> bool {
        if self.pushed_handles_from_network.contains(&id) {
            debug!(
                "Debouncing network handle change, was already pushed. {:?}",
                id
            );
            self.pushed_handles_from_network.remove(&id);
            return true;
        }
        false
    }

    pub(crate) fn apply_component_change_from_network(
        world: &mut World,
        e_id: Entity,
        name: String,
        data: &[u8],
    ) -> bool {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let component_data = bin_to_reflect(data, &registry);
        let name = if (*component_data).type_id() == TypeId::of::<SkinnedMeshSyncMapper>() {
            SkinnedMesh::default().reflect_type_path().to_string()
        } else {
            name
        };
        let component_data = if (*component_data).type_id() == TypeId::of::<SkinnedMeshSyncMapper>()
        {
            let component = component_data
                .downcast_ref::<SkinnedMeshSyncMapper>()
                .unwrap();
            SyncTrackerRes::to_skinned_mesh(world, component.clone()).clone_value()
        } else {
            component_data
        };
        let Some(registration) = registry.get_with_type_path(name.as_str()) else {
            debug!("Could not obtain registration for {:?}", name);
            return false;
        };
        let Some(reflect_component) = registration.data::<ReflectComponent>() else {
            debug!("Could not obtain reflect_component for {:?}", name);
            return false;
        };
        let Some(sync_entity) = world.entity(e_id).get::<SyncEntity>() else {
            debug!(
                "Could not find entity {:?} to apply comopnent change of type {:?}",
                e_id, name
            );
            return false;
        };
        let uuid = sync_entity.uuid;
        let previous_value = reflect_component.reflect(world.entity(e_id));
        let change_id = ComponentChangeId {
            id: uuid,
            name: name.to_string(),
        };
        if world
            .resource::<SyncTrackerRes>()
            .pushed_component_from_network
            .get(&change_id)
            .is_some()
        {
            debug!(
                "Skipped component from network, already pushed: {}v{} - {}",
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

    pub(crate) fn to_skinned_mapper(
        &self,
        assets: &Assets<SkinnedMeshInverseBindposes>,
        component: &SkinnedMesh,
    ) -> SkinnedMeshSyncMapper {
        let mut joints_uuid = Vec::<Uuid>::new();
        for e in &component.joints {
            if let Some(uuid) = self.entity_to_uuid.get(e) {
                joints_uuid.push(*uuid);
            }
        }
        let poses = assets.get(component.inverse_bindposes.id()).unwrap();
        let poses = (**poses).to_vec();
        SkinnedMeshSyncMapper {
            inverse_bindposes: poses,
            joints: joints_uuid,
        }
    }

    pub(crate) fn to_skinned_mesh(world: &mut World, mapper: SkinnedMeshSyncMapper) -> SkinnedMesh {
        world.resource_scope(
            |world, mut assets: Mut<Assets<SkinnedMeshInverseBindposes>>| {
                let tracker = world.resource::<SyncTrackerRes>();
                let mut joints = Vec::<Entity>::new();
                for uuid in &mapper.joints {
                    if let Some(e) = tracker.uuid_to_entity.get(uuid) {
                        joints.push(*e);
                    }
                }
                let poses = mapper.inverse_bindposes;
                let poses: SkinnedMeshInverseBindposes = poses.into();
                let handle = assets.add(poses);
                SkinnedMesh {
                    inverse_bindposes: handle,
                    joints,
                }
            },
        )
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

impl SyncComponent for App {
    fn sync_component<
        T: Component + TypePath + DynamicTypePath + Reflect + FromReflect + GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self {
        // application may try to setup sync without knowing if bevy_sync was enabled.
        if self.world().get_resource::<SyncTrackerRes>().is_none() {
            warn!("Trying to register sync_component in bevy_sync, but bevy_sync is not enabled.");
            return self;
        }

        self.register_type::<T>();
        self.register_type_data::<T, ReflectFromReflect>();
        let c_id = self.world_mut().init_component::<T>();
        let c_exclude_id = self.world_mut().init_component::<SyncExclude<T>>();
        let mut track = self.world_mut().resource_mut::<SyncTrackerRes>();
        track.registered_componets_for_sync.insert(c_id);
        track
            .sync_exclude_cid_of_component_cid
            .insert(c_id, c_exclude_id);
        if TypeId::of::<T>() == TypeId::of::<SkinnedMesh>() {
            self.add_systems(Update, sync_skinned_mesh);
        } else {
            self.add_systems(Update, sync_detect::<T>);
        }
        setup_cascade_registrations::<T>(self);

        self
    }

    fn sync_materials(&mut self, enable: bool) {
        let mut tracker = self.world_mut().resource_mut::<SyncTrackerRes>();
        tracker.sync_materials = enable;
    }

    fn sync_meshes(&mut self, enable: bool) {
        let mut tracker = self.world_mut().resource_mut::<SyncTrackerRes>();
        tracker.sync_meshes = enable;
    }

    fn sync_audios(&mut self, enable: bool) {
        let mut tracker = self.world_mut().resource_mut::<SyncTrackerRes>();
        tracker.sync_audios = enable;
    }
}

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component, Default)]
pub(crate) struct SkinnedMeshSyncMapper {
    pub inverse_bindposes: Vec<Mat4>,
    pub joints: Vec<Uuid>,
}

#[allow(clippy::type_complexity)]
fn sync_skinned_mesh(
    assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut tracker: ResMut<SyncTrackerRes>,
    q: Query<
        (&SyncEntity, &SkinnedMesh),
        (
            With<SyncEntity>,
            Without<SyncExclude<SkinnedMesh>>,
            Changed<SkinnedMesh>,
        ),
    >,
) {
    for (sup, component) in q.iter() {
        let component_to_send = tracker.to_skinned_mapper(&assets, component);
        tracker.signal_component_changed(sup.uuid, component_to_send.clone_value());
    }
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

fn setup_cascade_registrations<T: Component + Reflect + FromReflect + GetTypeRegistration>(
    app: &mut App,
) {
    if TypeId::of::<T>() == TypeId::of::<SkinnedMesh>() {
        app.register_type::<SkinnedMeshSyncMapper>();
    }

    if TypeId::of::<T>() == TypeId::of::<Mesh>() {
        app.register_type::<Image>();
        app.register_type::<Handle<Image>>();
    }

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
    if TypeId::of::<T>() == TypeId::of::<SpotLight>() {
        app.register_type::<Color>();
    }
    if TypeId::of::<T>() == TypeId::of::<DirectionalLight>() {
        app.register_type::<Color>();
    }
}

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InitialSyncFinished>();
        app.register_type::<SyncMark>();
        app.init_resource::<SyncTrackerRes>();
        app.add_plugins(BundleFixPlugin);
        app.add_plugins(ServerSyncPlugin);
        app.add_plugins(ClientSyncPlugin);
        app.init_state::<ServerState>();
        app.init_state::<ClientState>();
        app.add_event::<PromoteToHostEvent>();
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.parameters.clone());
        crate::networking::setup_server(app, self.parameters.clone());
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.parameters.clone());
        crate::networking::setup_client(app, self.parameters.clone());
    }
}
