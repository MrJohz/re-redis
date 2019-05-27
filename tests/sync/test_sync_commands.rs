extern crate reredis;

use reredis::{commands, SyncClient};

use std::ffi::OsStr;
use std::process::{Child, Command, Stdio};

struct RedisRunner {
    process: Child,
}

fn load_up_redis() -> RedisRunner {
    let process = Command::new("redis-server")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();

    RedisRunner { process }
}

#[test]
fn runs_redis() {
    /*let _ = */
    load_up_redis();

    let mut client = reredis::SyncClient::new("localhost:6379").unwrap();
    client.issue_command(commands::set("test-key", 32)).unwrap();
}

#[test]
fn runs_redis_as_well() {
    load_up_redis();
    let mut client = reredis::SyncClient::new("localhost:6379").unwrap();
    client.issue_command(commands::set("test-key", 42)).unwrap();
}
