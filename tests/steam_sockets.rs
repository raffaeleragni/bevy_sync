use serial_test::serial;
use setup::TestRun;

mod assert;
mod setup;

#[serial]
#[test]
fn test_steam_connect() {
    TestRun::default();
}
