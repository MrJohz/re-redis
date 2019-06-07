pub mod get;
pub use get::{get, mget};

pub mod set;
pub use set::{getset, mset, set};

pub mod increment;
pub use increment::{decr, decr_by, incr, incr_by};

pub mod bit_commands;
pub use bit_commands::{bitcount, getbit, setbit, bitpos};
