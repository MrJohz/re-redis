extern crate reredis;
mod utils;

use reredis::commands::*;

use crate::utils::load_redis_instance;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;
use std::thread;
use std::time::Duration;

#[test]
fn successfully_sets_and_gets_a_key_from_redis() {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();
    client.issue(set("test-key", 32)).unwrap();
    let value = client.issue(get::<i64>("test-key".into())).unwrap();
    assert_eq!(value, Some(32));
}

#[quickcheck]
fn qc_can_insert_an_arbitrary_key_and_integer_value_into_redis(key: String, value: i64) {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();
    client.issue(set(key.clone(), value)).unwrap();

    let returned_value = client.issue(get::<i64>(key)).unwrap();

    assert_eq!(returned_value, Some(value));
}

#[quickcheck]
fn qc_can_insert_an_arbitrary_key_with_a_timeout_into_redis(
    key: String,
    value: i64,
    timeout: u8,
) -> TestResult {
    if timeout == 0 {
        return TestResult::discard();
    }

    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();
    client
        .issue(set(key.clone(), value).with_expiry(Duration::from_secs(timeout as u64)))
        .unwrap();

    let returned_value = client.issue(get::<i64>(key)).unwrap();

    assert_eq!(returned_value, Some(value));
    TestResult::passed()
}

#[test]
fn inserting_a_key_with_a_timeout_expires_the_key() {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client
        .issue(set("test-key", 0).with_expiry(Duration::from_secs(1)))
        .unwrap();

    let returned = client.issue(get::<i64>("test-key".into())).unwrap();

    assert_eq!(Some(0), returned);

    thread::sleep(Duration::from_millis(1100));

    let returned = client.issue(get::<i64>("test-key".into())).unwrap();
    assert_eq!(None, returned);
}

#[test]
fn get_behaves_in_an_ergonomic_way_when_macros_arent_involved() {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("name", "Kevin")).unwrap();
    let world = client
        .issue(get("name".into()).with_default("World".to_string()))
        .unwrap();

    assert_eq!(format!("Hello, {}", world), "Hello, Kevin");
}

#[test]
fn inserting_multiple_keys_with_mset_sets_all_of_them_together() {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client
        .issue(
            mset()
                .add("this::that", 42)
                .add("the_other::that", 52)
                .add("all_them::that", 62),
        )
        .unwrap();

    assert_eq!(Some(42), client.issue(get("this::that".into())).unwrap());
    assert_eq!(
        Some(52),
        client.issue(get("the_other::that".into())).unwrap()
    );
    assert_eq!(
        Some(62),
        client.issue(get("all_them::that".into())).unwrap()
    );
}

#[test]
fn mset_can_fail_if_a_key_already_exists() {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("this::that", 100)).unwrap();

    let succeeded = client
        .issue(
            mset()
                .add("this::that", 42)
                .add("the_other::that", 52)
                .add("all_them::that", 62)
                .if_none_exists(),
        )
        .unwrap();

    assert_eq!(false, succeeded);

    assert_eq!(Some(100), client.issue(get("this::that".into())).unwrap());
}
