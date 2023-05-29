use bevy::{ecs::schedule::run_enter_schedule, prelude::*, utils::HashSet};
use bevy_renet::renet::{transport::NetcodeServerTransport, DefaultChannel, RenetServer};

use crate::{
    data::SyncTrackerRes, proto::Message, proto_serde::compo_to_bin, ServerState,
    SyncClientGeneratedEntity, SyncMark, SyncPusher,
};

use super::SyncDown;

pub(crate) struct ServerSendPlugin;

impl Plugin for ServerSendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();

        app.add_state::<ServerState>();
        app.add_systems(
            (
                server_disconnected
                    .run_if(state_exists_and_equals(ServerState::Connected))
                    .run_if(resource_removed::<NetcodeServerTransport>()),
                server_connected
                    .run_if(resource_added::<NetcodeServerTransport>())
                    .run_if(state_exists_and_equals(ServerState::Disconnected)),
            )
                .before(run_enter_schedule::<ServerState>)
                .in_base_set(CoreSet::StateTransitions),
        );

        app.add_system(server_reset.in_schedule(OnExit(ServerState::Connected)))
            .add_systems(
                (
                    reply_back_to_client_generated_entity,
                    entity_created_on_server,
                    entity_removed_from_server,
                    track_spawn_server,
                    react_on_changed_components,
                )
                    .chain()
                    .in_set(OnUpdate(ServerState::Connected)),
            );
    }
}

fn server_disconnected(mut state: ResMut<NextState<ServerState>>) {
    state.set(ServerState::Disconnected);
}

fn server_connected(mut state: ResMut<NextState<ServerState>>) {
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

fn reply_back_to_client_generated_entity(
    mut commands: Commands,
    opt_server: Option<ResMut<RenetServer>>,
    mut query: Query<(Entity, &SyncClientGeneratedEntity), Added<SyncClientGeneratedEntity>>,
) {
    let Some(mut server) = opt_server else { return };
    for (entity_id, marker_component) in query.iter_mut() {
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
    query: Query<Entity>,
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
    mut track: ResMut<SyncPusher>,
) {
    let Some(mut server) = opt_server else { return; };
    let registry = registry.clone();
    let registry = registry.read();
    while let Some(change) = track.components.pop_front() {
        for cid in server.clients_id().into_iter() {
            server.send_message(
                cid,
                DefaultChannel::ReliableOrdered,
                bincode::serialize(&Message::EntityComponentUpdated {
                    id: change.id,
                    name: change.name.clone(),
                    data: compo_to_bin(change.data.clone_value(), &registry),
                })
                .unwrap(),
            );
        }
    }
}

pub(crate) fn build_initial_sync(world: &World) -> Vec<Message> {
    let mut entity_ids_sent: HashSet<Entity> = HashSet::new();
    let mut result: Vec<Message> = Vec::new();
    let track = world.resource::<SyncTrackerRes>();
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    for arch in world
        .archetypes()
        .iter()
        .filter(|arch| track.is_synched_archetype(arch))
    {
        for c_id in arch
            .components()
            .filter(|&c_id| track.is_synched_component(&c_id))
        {
            let c_info = unsafe { world.components().get_info_unchecked(c_id) };
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
                if !entity_ids_sent.contains(&e_id) {
                    result.push(Message::EntitySpawn { id: e_id });
                    entity_ids_sent.insert(e_id);
                }
                result.push(Message::EntityComponentUpdated {
                    id: e_id,
                    name: type_name.into(),
                    data: compo_bin,
                });
            }
        }
    }

    result
}
