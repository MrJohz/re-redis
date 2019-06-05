use rand::Rng;
use std::io::Read;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use lazy_static::lazy_static;
use std::sync::Mutex;
lazy_static! {
    static ref GLOBAL_LOCK: Mutex<()> = Mutex::new(());
}

#[derive(Debug)]
pub struct RedisRunner {
    process: Child,
    connection_string: String,
    #[allow(dead_code)] // basically just keeping it around to prevent dropping
    data_dir: TempDir,
}

impl RedisRunner {
    pub fn address(&self) -> &str {
        &self.connection_string
    }
}

impl Drop for RedisRunner {
    fn drop(&mut self) {
        self.process.kill().unwrap_or_else(|err| {
            eprintln!("Warning - failed to kill process due to err: {}", err);
        });
    }
}

pub fn load_redis_instance() -> RedisRunner {
    let _lock = GLOBAL_LOCK.lock();
    let port = loop {
        let temp_port = rand::thread_rng().gen_range(6000, 6999);
        if TcpStream::connect(format!("localhost:{}", temp_port)).is_err() {
            break temp_port;
        }
    };

    let data_dir = tempfile::Builder::new()
        .prefix("redis-tests.")
        .tempdir()
        .unwrap();

    let mut process = Command::new("redis-server")
        .args(&["--port", &port.to_string()])
        .args(&["--dir", data_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let connection_string = format!("localhost:{}", port);

    loop {
        if process.try_wait().unwrap().is_some() {
            let mut stdout_buffer = String::new();
            process
                .stdout
                .unwrap()
                .read_to_string(&mut stdout_buffer)
                .unwrap();
            dbg!(stdout_buffer);
            panic!("redis-server has already closed, cannot connect to it")
        }
        if TcpStream::connect(&connection_string).is_err() {
            thread::sleep(Duration::from_millis(100));
        } else {
            break;
        }
    }

    RedisRunner {
        process,
        data_dir,
        connection_string,
    }
}
