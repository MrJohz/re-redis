use crate::sans_io::Client as SansIoClient;
use crate::{Command, RBytes, RedisError, StructuredCommand};
use std::io::{BufRead, BufReader, BufWriter, Result as IoResult, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;

#[derive(Debug)]
pub struct Client {
    writer: BufWriter<TcpStream>,
    parser: SansIoClient,
}

impl Client {
    pub fn new(address: impl ToSocketAddrs) -> IoResult<Self> {
        let stream = TcpStream::connect(address)?;
        let writer = BufWriter::new(stream.try_clone()?);
        let (parser, tx_bytes) = SansIoClient::new();

        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut buffer = Vec::new();

            loop {
                match reader.read_until(b'\n', &mut buffer) {
                    Ok(_) => {
                        let result = tx_bytes.send(Ok(buffer.clone().into()));
                        if result.is_err() {
                            break;
                        }
                        buffer.clear();
                    }
                    Err(err) => {
                        tx_bytes.send(Err(err)).unwrap();
                        break;
                    }
                }
            }

            // TODO: figure out why this closes when the tests are finished
            //   Or just generally figure out cleanup
        });

        Ok(Self { parser, writer })
    }

    pub fn with_auth<'a>(
        address: impl ToSocketAddrs,
        pass: impl Into<RBytes<'a>>,
    ) -> Result<Self, RedisError> {
        let mut client = Self::new(address).map_err(RedisError::ConnectionError)?;
        client.issue(Command::cmd("AUTH").with_arg(pass))?;
        Ok(client)
    }

    pub fn issue<Cmd>(&mut self, cmd: Cmd) -> Result<<Cmd as StructuredCommand>::Output, RedisError>
    where
        Cmd: StructuredCommand,
    {
        let bytes = self.parser.issue_command(&cmd);
        self.writer
            .write(&bytes)
            .map_err(RedisError::ConnectionError)?;
        self.writer.flush().map_err(RedisError::ConnectionError)?;
        self.parser.get_response(cmd)
    }
}
