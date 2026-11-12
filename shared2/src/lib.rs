#![no_std]

#[cfg(test)]
extern crate std;

pub const PROTOCOL_VERSION: &str = "6";
pub const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";

pub mod http;

mod fixed_str;
mod id;
mod message;
mod message_reader;
mod message_writer;
mod store;
mod token;

pub use fixed_str::{FixedStr, FixedStrError};
pub use id::ID;
pub use message::{Message, MessageDecodeError};
pub use message_reader::{MessageReader, MessageReaderError};
pub use message_writer::MessageWriter;
pub use store::Store;
pub use token::Token;
