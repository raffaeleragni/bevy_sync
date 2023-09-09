use std::{
    any::TypeId,
    collections::VecDeque,
    net::{IpAddr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::{
    asset::HandleId,
    ecs::component::ComponentId,
    prelude::{
        debug, AlphaMode, App, AppTypeRegistry, Assets, Changed, Color, Component, Entity, Handle,
        Image, ParallaxMappingMethod, Plugin, Query, ReflectComponent, Res, ResMut, Resource,
        StandardMaterial, Update, With, World,
    },
    reflect::{FromReflect, GetTypeRegistration, Reflect, ReflectFromReflect},
    utils::{HashMap, HashSet},
};

use bevy_renet::{
    renet::{
        transport::{
            ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport,
            ServerAuthentication, ServerConfig,
        },
        ConnectionConfig, RenetClient, RenetServer,
    },
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};

use crate::{
    client::ClientSyncPlugin, proto::PROTOCOL_ID, proto_serde::bin_to_compo,
    server::ServerSyncPlugin, ClientPlugin, ServerPlugin, SyncComponent, SyncDown, SyncMark,
    SyncPlugin, SyncUp,
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
    pub(crate) changed_components: VecDeque<ComponentChange>,
    pushed_component_from_network: HashSet<ComponentChangeId>,
    pushed_handles_from_network: HashSet<HandleId>,
    material_handles: HashMap<HandleId, Handle<StandardMaterial>>,
    sync_materials: bool,
}

impl SyncTrackerRes {
    pub(crate) fn signal_component_changed(&mut self, id: Entity, data: Box<dyn Reflect>) {
        let name = data.type_name().into();
        let change_id = ComponentChangeId { id, name };
        if self.pushed_component_from_network.contains(&change_id) {
            self.pushed_component_from_network.remove(&change_id);
            return;
        }
        self.changed_components
            .push_back(ComponentChange { change_id, data });
    }

    pub(crate) fn skip_network_handle_change(&mut self, id: HandleId) -> bool {
        if self.pushed_handles_from_network.contains(&id) {
            self.pushed_handles_from_network.remove(&id);
            return true;
        }
        false
    }

    pub(crate) fn apply_component_change_from_network(
        e_id: Entity,
        name: String,
        data: Vec<u8>,
        world: &mut World,
    ) -> bool {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let component_data = bin_to_compo(&data, &registry);
        let registration = registry.get_with_name(name.as_str()).unwrap();
        let reflect_component = registration.data::<ReflectComponent>().unwrap();
        let previous_value = reflect_component.reflect(world.entity(e_id));
        if SyncTrackerRes::needs_to_change(previous_value, &*component_data) {
            world
                .resource_mut::<SyncTrackerRes>()
                .pushed_component_from_network
                .insert(ComponentChangeId {
                    id: e_id,
                    name: name.clone(),
                });
            let entity = &mut world.entity_mut(e_id);
            reflect_component.apply_or_insert(entity, component_data.as_reflect());
            debug!(
                "Changed component from network: {}v{} - {}",
                e_id.index(),
                e_id.generation(),
                name.clone()
            );
            true
        } else {
            debug!(
                "Skipped component from network: {}v{} - {}",
                e_id.index(),
                e_id.generation(),
                name.clone()
            );
            false
        }
    }

    pub(crate) fn apply_material_change_from_network(
        id: HandleId,
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
        let handle = materials.set(id, mat);
        // need to keep a reference somewhere else the material will be destroyed right away
        world
            .resource_mut::<SyncTrackerRes>()
            .material_handles
            .insert(id, handle);
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
}

#[allow(clippy::type_complexity)]
fn sync_detect_server<T: Component + Reflect>(
    mut push: ResMut<SyncTrackerRes>,
    q: Query<(Entity, &T), (With<SyncDown>, Changed<T>)>,
) {
    for (e_id, component) in q.iter() {
        push.signal_component_changed(e_id, component.clone_value());
    }
}

#[allow(clippy::type_complexity)]
fn sync_detect_client<T: Component + Reflect>(
    mut push: ResMut<SyncTrackerRes>,
    q: Query<(&SyncUp, &T), (With<SyncUp>, Changed<T>)>,
) {
    for (sup, component) in q.iter() {
        push.signal_component_changed(sup.server_entity_id, component.clone_value());
    }
}

impl SyncComponent for App {
    fn sync_component<T: Component + Reflect + FromReflect + GetTypeRegistration>(
        &mut self,
    ) -> &mut Self {
        self.register_type::<T>();
        self.register_type_data::<T, ReflectFromReflect>();
        let c_id = self.world.init_component::<T>();
        let mut data = self.world.resource_mut::<SyncTrackerRes>();
        data.sync_components.insert(c_id);
        self.add_systems(Update, sync_detect_server::<T>);
        self.add_systems(Update, sync_detect_client::<T>);

        if TypeId::of::<T>() == TypeId::of::<Handle<StandardMaterial>>() {
            self.register_type_data::<StandardMaterial, ReflectFromReflect>();
            self.register_type::<Color>();
            self.register_type::<Image>();
            self.register_type::<Handle<Image>>();
            self.register_type::<Option<Handle<Image>>>();
            self.register_type::<AlphaMode>();
            self.register_type::<ParallaxMappingMethod>();
        }

        self
    }

    fn sync_materials(&mut self, enable: bool) {
        let mut tracker = self.world.resource_mut::<SyncTrackerRes>();
        tracker.sync_materials = enable;
    }
}

#[derive(Component)]
pub(crate) struct SyncClientGeneratedEntity {
    pub(crate) client_id: u64,
    pub(crate) client_entity_id: Entity,
}

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SyncMark>();
        app.init_resource::<SyncTrackerRes>();
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetServerPlugin);
        app.insert_resource(RenetServer::new(ConnectionConfig::default()));
        app.add_plugins(NetcodeServerPlugin);
        app.insert_resource(create_server(self.ip, self.port));

        app.add_plugins(ServerSyncPlugin);
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetClientPlugin);
        app.insert_resource(RenetClient::new(ConnectionConfig::default()));
        app.add_plugins(NetcodeClientPlugin);
        app.insert_resource(create_client(self.ip, self.port));

        app.add_plugins(ClientSyncPlugin);
    }
}

fn create_server(ip: IpAddr, port: u16) -> NetcodeServerTransport {
    let socket = UdpSocket::bind((ip, port)).unwrap();
    let server_addr = socket.local_addr().unwrap();
    const MAX_CLIENTS: usize = 64;
    let server_config = ServerConfig {
        max_clients: MAX_CLIENTS,
        protocol_id: PROTOCOL_ID,
        public_addr: server_addr,
        authentication: ServerAuthentication::Unsecure,
    };
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    NetcodeServerTransport::new(current_time, server_config, socket).unwrap()
}

fn create_client(ip: IpAddr, port: u16) -> NetcodeClientTransport {
    let socket = UdpSocket::bind((ip, 0)).unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = now.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        server_addr: SocketAddr::new(ip, port),
        protocol_id: PROTOCOL_ID,
        user_data: None,
    };
    NetcodeClientTransport::new(now, authentication, socket).unwrap()
}

pub(crate) fn sync_material_enabled(tracker: Res<SyncTrackerRes>) -> bool {
    tracker.sync_materials
}
