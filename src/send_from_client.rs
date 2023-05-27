use bevy::{
    prelude::{Added, App, Entity, Plugin, Query, ResMut},
    utils::HashSet,
};
use bevy_renet::renet::{DefaultChannel, RenetClient};

use crate::{data::SyncTrackerRes, proto::Message, SyncMark, SyncUp};

pub(crate) struct ClientSendPlugin;
impl Plugin for ClientSendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncTrackerRes>();
        app.add_system(track_spawn_client);
        app.add_system(entity_created_on_client);
        app.add_system(entity_removed_from_client);
    }
}

fn track_spawn_client(
    mut track: ResMut<SyncTrackerRes>,
    query: Query<(Entity, &SyncUp), Added<SyncUp>>,
) {
    for (e_id, sync_up) in query.iter() {
        track
            .server_to_client_entities
            .insert(sync_up.server_entity_id, e_id);
    }
}

fn entity_created_on_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut query: Query<Entity, Added<SyncMark>>,
) {
    let Some(mut client) = opt_client else { return };
    for id in query.iter_mut() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntitySpawn { id }).unwrap(),
        );
    }
}

fn entity_removed_from_client(
    opt_client: Option<ResMut<RenetClient>>,
    mut track: ResMut<SyncTrackerRes>,
    query: Query<Entity>,
) {
    let mut despawned_entities = HashSet::new();
    track
        .server_to_client_entities
        .retain(|&s_e_id, &mut e_id| {
            if query.get(e_id).is_err() {
                despawned_entities.insert(s_e_id);
                false
            } else {
                true
            }
        });
    let Some(mut client) = opt_client else { return };
    for &id in despawned_entities.iter() {
        client.send_message(
            DefaultChannel::ReliableOrdered,
            bincode::serialize(&Message::EntityDelete { id }).unwrap(),
        );
    }
}
