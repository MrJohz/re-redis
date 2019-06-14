#![cfg(feature = "async-client")]
#![feature(async_await)]

extern crate reredis;
mod utils;

use reredis::commands::*;

use crate::utils::*;

#[runtime::test]
async fn can_ping_the_default_server() {
    let server = load_redis_instance();
    let mut client = reredis::AsyncClient::new(server.address()).await.unwrap();

    assert_eq!((), client.issue(ping()).unwrap());
}

#[runtime::test]
async fn can_login_with_authorisation() {
    let server = RedisInstance::new()
        .with_setting("requirepass", &["\"test password\""])
        .build();
    let mut client = reredis::AsyncClient::with_auth(server.address(), "test password").await.unwrap();

    assert_eq!((), client.issue(ping()).unwrap());
}
