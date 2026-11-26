#![no_std]

#[cfg(test)]
extern crate std;

pub const PROTOCOL_VERSION: &str = "6";
pub const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";

pub mod env;
pub mod http;

mod fixed_str;
mod format_buffer;
mod id;
mod logger;
mod message;
mod message_reader;
mod message_writer;
mod stderr;
mod store;
mod token;

pub use fixed_str::{FixedStr, FixedStrError};
pub use format_buffer::FormatBuffer;
pub use id::ID;
pub use logger::{Logger, LoggerError};
pub use message::{MAX_TEXT_LENGTH, Message, MessageDecodeError};
pub use message_reader::{MessageReader, MessageReaderError};
pub use message_writer::MessageWriter;
pub use stderr::Stderr;
pub use store::Store;
pub use token::Token;
