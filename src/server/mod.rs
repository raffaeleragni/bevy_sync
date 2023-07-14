use bevy::{prelude::*, reflect::Reflect, utils::HashSet};
use bevy_renet::renet::{
    transport::NetcodeServerTransport, DefaultChannel, RenetServer, ServerEvent,
};

use crate::{
    lib_priv::{SyncClientGeneratedEntity, SyncTrackerRes},
    proto::Message,
    proto_serde::compo_to_bin,
    ServerState, SyncMark,
};

use super::SyncDown;

mod receiver;

pub(crate) struct ServerSyncPlugin;

impl Plugin for ServerSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();

        app.add_state::<ServerState>();
        app.add_systems(
            Update,
            server_connected
                .run_if(state_exists_and_equals(ServerState::Disconnected))
                .run_if(resource_added::<NetcodeServerTransport>()),
        );
        app.add_systems(
            Update,
            server_disconnected
                .run_if(state_exists_and_equals(ServerState::Connected))
                .run_if(resource_removed::<NetcodeServerTransport>()),
        );

        app.add_systems(OnExit(ServerState::Connected), server_reset);
        app.add_systems(
            Update,
            (
                reply_back_to_client_generated_entity,
                entity_created_on_server,
                entity_parented_on_server,
                entity_removed_from_server,
                track_spawn_server,
                react_on_changed_components,
                react_on_changed_materials,
            )
                .chain()
                .run_if(state_exists_and_equals(ServerState::Connected)),
        );
        app.add_systems(
            Update,
            (client_connected, receiver::poll_for_messages)
                .chain()
                .run_if(state_exists_and_equals(ServerState::Connected)),
        );
    }
}

fn client_connected(mut cmd: Commands, mut server_events: EventReader<ServerEvent>) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Client connected with client id: {}", client_id);
                let c_id = *client_id;
                cmd.add(move |world: &mut World| send_initial_sync(c_id, world));
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!(
                    "Client disconnected with client id: {}, reason: {}",
                    client_id, reason
                );
            }
        }
    }
}

fn server_disconnected(mut state: ResMut<NextState<ServerState>>) {
    info!("Server is shut down.");
    state.set(ServerState::Disconnected);
}

fn server_connected(mut state: ResMut<NextState<ServerState>>) {
    info!("Server ready to accept connections.");
    state.set(ServerState::Connected);
}

fn track_spawn_server(mut track: ResMut<SyncTrackerRes>, query: Query<Entity, Added<SyncDown>>) {
    for e_id in query.iter() {
        track.server_to_client_entities.insert(e_id, e_id);
    }
}

fn server_reset(mut cmd: Commands) {
    cmd.insert_resource(SyncTrackerRes::default());
}

fn entity_created_on_server(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    let Some(mut server) = opt_server else { return };
    for id in query.iter_mut() {
        debug!(
            "New entity created on server: {}v{}",
            id.index(),
            id.generation()
        );
        for client_id in server.clients_id().into_iter() {
            server.send_message(
                client_id,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
            );
        }
        let mut entity = commands.entity(id);
        entity.remove::<SyncMark>().insert(SyncDown {});
    }
}

fn entity_parented_on_server(
    opt_server: Option<ResMut<RenetServer>>,
    query: Query<(Entity, &Parent), Changed<Parent>>,
) {
    let Some(mut server) = opt_server else { return };
    for (e_id, p) in query.iter() {
        for client_id in server.clients_id().into_iter() {
            server.send_message(
                client_id,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityParented {
                    server_entity_id: e_id,
                    server_parent_id: p.get(),
                })
                .unwrap(),
            );
        }
    }
}

fn reply_back_to_client_generated_entity(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<(Entity, &SyncClientGeneratedEntity), Added<SyncClientGeneratedEntity>>,
) {
    let Some(mut server) = opt_server else { return };
    for (entity_id, marker_component) in query.iter_mut() {
        debug!(
            "Replying to client generated entity for: {}v{}",
            entity_id.index(),
            entity_id.generation()
        );
        server.send_message(
            marker_component.client_id,
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntitySpawnBack {
                server_entity_id: entity_id,
                client_entity_id: marker_component.client_entity_id,
            })
            .unwrap(),
        );
        for cid in server.clients_id().into_iter() {
            if marker_component.client_id != cid {
                server.send_message(
                    cid,
                    DefaultChannel::ReliableOrdered,
                    bincode::serialize(&Message::EntitySpawn { id: entity_id }).unwrap(),
                );
            }
        }
        let mut entity = commands.entity(entity_id);
        entity
            .remove::<SyncClientGeneratedEntity>()
            .insert(SyncDown {});
    }
}

fn entity_removed_from_server(
    opt_server: Option<ResMut<RenetServer>>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity, With<SyncDown>>,
) {
    let mut despawned_entities = HashSet::new();
    track.server_to_client_entities.retain(|&e_id, _| {
        if query.get(e_id).is_err() {
            despawned_entities.insert(e_id);
            false
        } else {
            true
        }
    });
    let Some(mut server) = opt_server else { return };
    for &id in despawned_entities.iter() {
        debug!(
            "Entity was removed from server: {}v{}",
            id.index(),
            id.generation()
        );
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityDelete { id }).unwrap(),
            );
        }
    }
}

fn react_on_changed_components(
    registry: Res<AppTypeRegistry>,
    opt_server: Option<ResMut<RenetServer>>,
    mut track: ResMut<SyncTrackerRes>,
) {
    let Some(mut server) = opt_server else { return; };
    let registry = registry.clone();
    let registry = registry.read();
    while let Some(change) = track.changed_components.pop_front() {
        debug!(
            "Component was changed on server: {}",
            change.data.type_name()
        );
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::ComponentUpdated {
                    id: change.change_id.id,
                    name: change.change_id.name.clone(),
                    data: compo_to_bin(change.data.clone_value(), &registry),
                })
                .unwrap(),
            );
        }
    }
}

fn react_on_changed_materials(
    registry: Res<AppTypeRegistry>,
    opt_server: Option<ResMut<RenetServer>>,
    materials: Res<Assets<StandardMaterial>>,
    mut events: EventReader<AssetEvent<StandardMaterial>>,
) {
    let Some(mut server) = opt_server else { return; };
    let registry = registry.clone();
    let registry = registry.read();
    for event in &mut events {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                let Some(material) = materials.get(handle) else { return; };
                for cid in server.clients_id().into_iter() {
                    server.send_message(
                        cid,
                        DefaultChannel::ReliableOrdered,
                        bincode::serialize(&Message::StandardMaterialUpdated {
                            id: handle.id(),
                            material: compo_to_bin(material.clone_value(), &registry),
                        })
                        .unwrap(),
                    );
                }
            }
            AssetEvent::Removed { handle: _ } => {}
        }
    }
}

fn send_initial_sync(client_id: u64, world: &mut World) {
    info!("Sending initial sync to client id: {}", client_id);
    // exclusive access to world while looping through all objects, this can be blocking/freezing for the server
    let mut initial_sync = build_initial_sync(world);
    let mut server = world.resource_mut::<RenetServer>();
    debug!("Initial sync size: {}", initial_sync.len());
    for msg in initial_sync.drain(..) {
        let msg_bin = bincode::serialize(&msg).unwrap();
        server.send_message(client_id, DefaultChannel::ReliableOrdered, msg_bin);
    }
}

fn build_initial_sync(world: &World) -> Vec<Message> {
    let mut entity_ids_sent: HashSet<Entity> = HashSet::new();
    let mut result: Vec<Message> = Vec::new();
    let track = world.resource::<SyncTrackerRes>();
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let sync_down_id = world
        .component_id::<SyncDown>()
        .expect("SyncDown is not registered");
    for arch in world
        .archetypes()
        .iter()
        .filter(|arch| arch.contains(sync_down_id))
    {
        for arch_entity in arch.entities() {
            let entity = world.entity(arch_entity.entity());
            let e_id = entity.id();
            if !entity_ids_sent.contains(&e_id) {
                result.push(Message::EntitySpawn { id: e_id });
                entity_ids_sent.insert(e_id);
            }
        }
        for c_id in arch
            .components()
            .filter(|&c_id| track.sync_components.contains(&c_id))
        {
            let c_info = world
                .components()
                .get_info(c_id)
                .expect("component not found");
            let type_name = c_info.name();
            let registration = registry
                .get(c_info.type_id().expect("not registered"))
                .expect("not registered");
            let reflect_component = registration
                .data::<ReflectComponent>()
                .expect("missing #[reflect(Component)]");
            for arch_entity in arch.entities() {
                let entity = world.entity(arch_entity.entity());
                let e_id = entity.id();
                let component = reflect_component.reflect(entity).expect("not registered");
                let compo_bin = compo_to_bin(component.clone_value(), &registry);
                result.push(Message::ComponentUpdated {
                    id: e_id,
                    name: type_name.into(),
                    data: compo_bin,
                });
            }
        }
    }

    result
}
