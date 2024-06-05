use bevy::app::App;
use bevy_renet::renet::{transport::NetcodeServerTransport, RenetServer};
use bevy_sync::{PromoteToHostEvent, SyncMark};
use serial_test::serial;
use setup::TestRun;

mod assert;
mod setup;

#[test]
#[serial]
fn test_host_promotion_with_one_client() {
    TestRun::default().run(
        2,
        TestRun::no_pre_setup,
        |env| {
            assert!(is_host(&mut env.server));
            let server = env.server.world.resource_mut::<RenetServer>();
            let event = PromoteToHostEvent {
                id: server.clients_id().first().unwrap().to_owned(),
            };
            env.server.world.send_event(event);
            env.update(10);
            assert!(!is_host(&mut env.server));
            env.server.world.spawn(SyncMark {});
            1
        },
        assert::entities_in_sync,
    );
}

fn is_host(app: &mut App) -> bool {
    app.world.get_resource::<NetcodeServerTransport>().is_some()
}
