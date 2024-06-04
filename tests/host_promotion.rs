use bevy_renet::renet::ClientId;
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
            let event = PromoteToHostEvent {
                id: ClientId::from_raw(1),
            };
            env.server.world.send_event(event);
            env.update(10);
            env.server.world.spawn(SyncMark {});
            1
        },
        assert::entities_in_sync,
    );
}
