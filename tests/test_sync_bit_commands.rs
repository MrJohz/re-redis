extern crate reredis;
mod utils;

use reredis::commands::*;

use crate::utils::load_redis_instance;
use quickcheck_macros::quickcheck;
use std::collections::HashMap;

#[test]
fn getbit_and_setbit_work_on_a_single_bit() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    let oldbit = client.issue(setbit("test-key", 100, true)).unwrap();
    assert_eq!(false, oldbit);

    assert_eq!(true, client.issue(getbit("test-key", 100)).unwrap());
}

#[quickcheck]
fn qc_can_getbit_and_setbit_from_a_key_multiple_times(pairs: Vec<(u32, bool)>) {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    let mut oldbits = HashMap::new();

    for (pos, newbit) in pairs {
        let oldbit = client.issue(setbit("test-key", pos, newbit)).unwrap();
        let expected_oldbit = oldbits.get(&pos).unwrap_or(&false);

        assert_eq!(*expected_oldbit, oldbit);
        assert_eq!(newbit, client.issue(getbit("test-key", pos)).unwrap());
        oldbits.insert(pos, newbit);
    }
}

#[test]
fn bitcount_can_get_the_count_of_all_bits_in_a_string() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", "foobar")).unwrap();
    assert_eq!(26, client.issue(bitcount("mykey")).unwrap());
}

#[test]
fn bitcount_can_get_the_count_of_some_bits_in_a_string() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", "foobar")).unwrap();
    assert_eq!(10, client.issue(bitcount("mykey").in_range(0..1)).unwrap());
}

#[test]
fn bitcount_can_get_the_count_of_the_last_bits_in_a_string() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", "foobar")).unwrap();
    assert_eq!(7, client.issue(bitcount("mykey").in_range(-2..-1)).unwrap());
}

#[test]
fn bitcount_can_use_exclusive_upper_ranges_if_desired() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", "foobar")).unwrap();
    assert_eq!(10, client.issue(bitcount("mykey").in_range(0..=2)).unwrap());
}

#[test]
fn bitpos_returns_none_if_no_bits_in_a_string_are_set() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", "\x00")).unwrap();
    assert_eq!(None, client.issue(bitpos("mykey", true)).unwrap());
}

#[test]
fn bitpos_returns_the_first_set_bit_in_a_string() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", "\x10")).unwrap();
    assert_eq!(Some(3), client.issue(bitpos("mykey", true)).unwrap());
}

#[test]
fn bitpos_returns_the_first_unset_bit_in_a_string() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("mykey", b"\xFF\xF0".as_ref())).unwrap();
    assert_eq!(Some(12), client.issue(bitpos("mykey", false)).unwrap());
}
