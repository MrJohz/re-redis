extern crate reredis;

use reredis::commands;

use self::reredis::types::commands::Get;
use lazy_static::lazy_static;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;
use rand::Rng;
use std::net::{TcpStream, ToSocketAddrs};
use std::ops::DerefMut;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

//lazy_static! {
//    static ref REDIS_INSTANCES: Mutex<Vec<Arc<Mutex<RedisRunner>>>> = Mutex::new(Vec::new());
//}

struct RedisRunner {
    process: Child,
    connection_string: String,
    #[allow(dead_code)] // basically just keeping it around to prevent dropping
    data_dir: TempDir,
}

impl RedisRunner {
    fn address(&self) -> &str {
        &self.connection_string
    }
    fn wait_for_connection(&mut self) {
        loop {
            if self.process.try_wait().unwrap().is_some() {
                panic!("redis-server has already closed, cannot connect to it")
            }
            if TcpStream::connect(&self.connection_string).is_err() {
                thread::sleep(Duration::from_millis(100));
            } else {
                return;
            }
        }
    }
}

impl Drop for RedisRunner {
    fn drop(&mut self) {
        self.process.kill().expect("failed to kill the process");
    }
}

fn load_up_redis() -> RedisRunner {
    let port = rand::thread_rng().gen_range(6000, 6500);
    let data_dir = tempfile::Builder::new()
        .prefix("redis-tests.")
        .tempdir()
        .unwrap();

    let process = Command::new("redis-server")
        .args(&["--port", &port.to_string()])
        .args(&["--dir", data_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let connection_string = format!("localhost:{}", port);

    RedisRunner {
        process,
        data_dir,
        connection_string,
    }
}

#[test]
fn successfully_sets_and_gets_a_key_from_redis() {
    let mut server = load_up_redis();
    server.wait_for_connection();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();
    client.issue_command(commands::set("test-key", 32)).unwrap();
    let value = client
        .issue_command::<Get<Option<i64>>>(commands::get("test-key"))
        .unwrap();
    assert_eq!(value, Some(32));
}

#[quickcheck]
fn qc_can_insert_an_arbitrary_key_and_integer_value_into_redis(key: String, value: i64) {
    let mut server = load_up_redis();
    server.wait_for_connection();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();
    client
        .issue_command(commands::set(key.clone(), value))
        .unwrap();

    let returned_value = client
        .issue_command::<Get<Option<i64>>>(commands::get(key))
        .unwrap();

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

    let mut server = load_up_redis();
    server.wait_for_connection();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();
    client
        .issue_command(
            commands::set(key.clone(), value).with_expiry(Duration::from_millis(timeout as u64)),
        )
        .unwrap();

    let returned_value = client
        .issue_command::<Get<Option<i64>>>(commands::get(key))
        .unwrap();

    assert_eq!(returned_value, Some(value));
    TestResult::passed()
}

#[test]
fn inserting_a_key_with_a_timeout_expires_the_key() {
    let mut server = load_up_redis();
    server.wait_for_connection();

    let mut client = reredis::SyncClient::new(server.address()).unwrap();

    client
        .issue_command(commands::set("test-key", 0).with_expiry(Duration::from_secs(2)))
        .unwrap();

    let returned = client
        .issue_command::<Get<Option<i64>>>(commands::get("test-key"))
        .unwrap();

    assert_eq!(Some(0), returned);

    thread::sleep(Duration::from_millis(2200));

    let returned = client
        .issue_command::<Get<Option<u64>>>(commands::get("test-key"))
        .unwrap();
    assert_eq!(None, returned);
}
