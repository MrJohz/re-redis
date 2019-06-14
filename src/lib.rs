#![cfg_attr(feature = "async-client", feature(async_await))]

pub mod sans_io;
pub use sans_io::Client as SansIoClient;

pub mod types;
pub use types::*;

#[cfg(feature = "async-client")]
pub mod async_client;
#[cfg(feature = "async-client")]
pub use async_client::Client as AsyncClient;

#[cfg(feature = "sync-client")]
pub mod sync_client;
#[cfg(feature = "sync-client")]
pub use sync_client::Client as SyncClient;
