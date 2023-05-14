mod test_setup;
use super::*;
use serial_test::serial;
use test_setup::TestEnv;

#[test]
#[serial]
fn test_connection_setup() {
    TestEnv::default().run(|_, _| {}, |_, _| {});
}

#[test]
#[serial]
fn test_entity_copied() {
    let setup = |s: &mut App, c: &mut App| {
        assert_eq!(s.world.entities().is_empty(), true);
        assert_eq!(c.world.entities().is_empty(), true);

        s.world.spawn(SyncUp::default());
    };

    let assertion = |s: &mut App, c: &mut App| {
        assert_eq!(s.world.entities().is_empty(), false);
        assert_eq!(c.world.entities().is_empty(), false);
    };

    TestEnv::default().run(setup, assertion);
}
