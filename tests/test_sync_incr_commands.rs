extern crate reredis;
mod utils;

use reredis::commands::*;

use crate::utils::load_redis_instance;
use quickcheck_macros::quickcheck;

#[quickcheck]
fn qc_incr_by_can_increment_a_key_by_an_arbitrary_integer(n: i64) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    assert_eq!(n, client.issue(incr_by("my-key", n)).unwrap());
    assert_eq!(Some(n), client.issue(get("my-key")).unwrap());
}

#[quickcheck]
fn qc_incr_can_increment_a_key_by_1_an_arbitrary_number_of_times(n: u32) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    for _ in 0..n {
        client.issue(incr("my-key")).unwrap();
    }

    assert_eq!(Some(n), client.issue(get("my-key")).unwrap());
}

#[quickcheck]
fn qc_decr_by_can_decrement_a_key_by_an_arbitrary_integer(n: i64) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    assert_eq!(-n, client.issue(decr_by("my-key", n)).unwrap());
    assert_eq!(Some(-n), client.issue(get("my-key")).unwrap());
}

#[test]
fn decr_can_decrement_a_key_once() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    assert_eq!(-1, client.issue(decr("my-key")).unwrap());
    assert_eq!(Some(-1), client.issue(get("my-key")).unwrap());
}

#[quickcheck]
fn qc_decr_can_decrement_a_key_by_1_an_arbitrary_number_of_times(n: u32) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    for _ in 0..n {
        client.issue(decr("my-key")).unwrap();
    }

    assert_eq!(Some(-(n as i64)), client.issue(get("my-key")).unwrap());
}

#[quickcheck]
fn qc_incr_by_float_can_increment_by_an_arbitrary_floating_point_value(n: f64) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    assert_eq!(n, client.issue(incr_by_float("my-key", n)).unwrap());

    assert_eq!(Some(n), client.issue(get("my-key")).unwrap());
}

#[quickcheck]
fn qc_decr_by_float_can_decrement_by_an_arbitrary_floating_point_value(n: f64) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", 0)).unwrap();

    assert_eq!(-n, client.issue(decr_by_float("my-key", n)).unwrap());

    assert_eq!(Some(-n), client.issue(get("my-key")).unwrap());
}
