extern crate reredis;
mod utils;

use reredis::commands::*;

use quickcheck::TestResult;
use quickcheck_macros::quickcheck;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use utils::load_redis_instance;

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

#[test]
fn mget_returns_a_list_of_keys_with_the_same_type() {
    let server = load_redis_instance();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("this", 100)).unwrap();
    client.issue(set("that", 100)).unwrap();
    client.issue(set("other", 100)).unwrap();

    assert_eq!(
        vec![Some(100), Some(100), Some(100)],
        client
            .issue(mget().with_keys(vec!["this", "that", "other"]))
            .unwrap()
    )
}

#[quickcheck]
fn qc_mget_and_mset_can_work_together(pairs: Vec<(String, i64)>) -> TestResult {
    if pairs.len() == 0 {
        return TestResult::discard();
    }
    let duplicate_check = pairs.iter().map(|(key, _)| key).collect::<HashSet<_>>();
    if duplicate_check.len() != pairs.len() {
        // Weird things happen if we have duplicate keys (quite naturally, but that's not
        // something we want to test here).
        return TestResult::discard();
    }

    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    let insertable: Vec<(String, i64)> = pairs.clone();
    let keys: Vec<String> = pairs.clone().into_iter().map(|(key, _)| key).collect();
    let values: Vec<Option<i64>> = pairs
        .clone()
        .into_iter()
        .map(|(_, value)| Some(value))
        .collect();

    client.issue(mset().with_pairs(insertable)).unwrap();

    assert_eq!(values, client.issue(mget().with_keys(keys)).unwrap());

    TestResult::passed()
}

#[test]
fn getset_returns_null_if_the_key_doesnt_exist_yet() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    let value = client.issue(getset::<i64, _, _>("test-key", 120)).unwrap();
    assert_eq!(value, None);
}

#[test]
fn getset_returns_a_value_if_the_key_has_previously_been_set() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("test-key", "this is a value")).unwrap();

    let value = client.issue(getset("test-key", 120)).unwrap();
    assert_eq!(value, Some("this is a value".to_string()));
}
