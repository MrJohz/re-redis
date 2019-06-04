use rand::Rng;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

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
        self.process.kill().unwrap_or_else(|err| {
            eprintln!("Warning - failed to kill process due to err: {}", err);
        });
    }
}

pub fn load_redis_instance() -> RedisRunner {
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

    let process = Command::new("redis-server")
        .args(&["--port", &port.to_string()])
        .args(&["--dir", data_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let connection_string = format!("localhost:{}", port);

    let mut runner = RedisRunner {
        process,
        data_dir,
        connection_string,
    };

    runner.wait_for_connection();
    runner
}
