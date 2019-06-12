extern crate reredis;
mod utils;

use reredis::commands::*;

use crate::utils::load_redis_instance;

#[test]
fn can_send_and_receive_non_utf8_bytes() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", b"\x00\xFF")).unwrap();

    assert_eq!(Some(vec![0x00, 0xFF]), client.issue(get("my-key")).unwrap());
}
