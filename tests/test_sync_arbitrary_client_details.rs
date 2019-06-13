extern crate reredis;
mod utils;

use reredis::commands::*;

use crate::utils::*;

#[test]
fn can_send_and_receive_non_utf8_bytes() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client.issue(set("my-key", b"\x00\xFF")).unwrap();

    assert_eq!(Some(vec![0x00, 0xFF]), client.issue(get("my-key")).unwrap());
}

#[test]
fn can_login_with_authorisation() {
    let server = RedisInstance::new()
        .with_setting("requirepass", &["\"test password\""])
        .build();
    let mut client = reredis::SyncClient::with_auth(server.address(), "test password").unwrap();

    assert_eq!((), client.issue(ping()).unwrap());
}

#[test]
fn can_emit_echo_command() {
    let server = load_redis_instance();
    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    assert_eq!("test", client.issue(echo("test")).unwrap());
}
