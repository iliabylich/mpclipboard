#![no_std]
#![forbid(unsafe_code)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::std_instead_of_core)]
#![expect(clippy::missing_errors_doc)]
#![expect(clippy::redundant_pub_crate)]
#![allow(clippy::option_if_let_else)]
#![doc = include_str!("../README.md")]

#[cfg(test)]
extern crate alloc;

mod config;
pub use config::{ConfigParser, ConfigParserError};

mod array_writer;
mod line_reader;
pub use line_reader::LineReaderError;

mod upgrade_request;
mod upgrade_request_reader;
mod upgrade_request_writer;
pub use self::{
    upgrade_request::UpgradeRequest,
    upgrade_request_reader::{UpgradeRequestReader, UpgradeRequestReaderError},
    upgrade_request_writer::{UpgradeRequestWriter, UpgradeRequestWriterError},
};

mod upgrade_response;
mod upgrade_response_reader;
mod upgrade_response_writer;
pub use self::{
    upgrade_response_reader::{UpgradeResponseReader, UpgradeResponseReaderError},
    upgrade_response_writer::{UpgradeResponseWriter, UpgradeResponseWriterError},
};

mod message;
mod message_reader;
mod message_writer;
pub use self::{
    message::{Message, MessageError},
    message_reader::MessageReader,
    message_writer::{MessageWriter, MessageWriterError},
};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod timerfd;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use timerfd::Timerfd;

pub(crate) const MAX_HOST_LENGTH: usize = 249;
const MAX_PORT_LENGTH: usize = 5;
pub(crate) const MAX_HOST_PORT_LENGTH: usize = MAX_HOST_LENGTH + 1 + MAX_PORT_LENGTH;
const _: () = assert!(MAX_HOST_PORT_LENGTH == 255);

pub type HostPort = NonEmptyInlineString<MAX_HOST_PORT_LENGTH>;

const MAX_TOKEN_LENGTH: usize = 100;
pub type Token = NonEmptyInlineString<MAX_TOKEN_LENGTH>;

pub(crate) const MAX_ID_LENGTH: usize = 100;
pub type ID = NonEmptyInlineString<MAX_ID_LENGTH>;

const MAX_VERSION_LENGTH: usize = 50;
pub type Version = NonEmptyInlineString<MAX_VERSION_LENGTH>;

pub(crate) const START_LINE: &str = "GET / HTTP/1.1";
pub(crate) const HOST_PREFIX: &str = "Host: ";
pub(crate) const TOKEN_PREFIX: &str = "Token: ";
pub(crate) const ID_PREFIX: &str = "ID: ";
pub(crate) const VERSION_PREFIX: &str = "Version: ";
pub(crate) const CONNECTION_UPGRADE_HEADER: &str = "Connection: Upgrade";
pub(crate) const UPGRADE_MPCLIPBOARD_RAW_HEADER: &str = "Upgrade: mpclipboard-raw";

mod non_empty_inline_string;
pub use non_empty_inline_string::{NonEmptyInlineString, NonEmptyInlineStringError};

mod buffer;
pub use buffer::Buffer;

mod wants;
pub use wants::{OptionWantsExt, Wants};

mod event_loop;
pub use event_loop::{Epoch, EventLoop, EventLoopError, EventLoopFdResult, EventLoopResult};

mod revents;
pub use revents::{REvents, REventsError};

mod store;
pub use store::Store;

mod tcp_keep_alive;
pub use tcp_keep_alive::{TcpKeepAliveError, enable_tcp_keep_alive};

mod url;
pub use url::{Url, UrlParseError};

pub(crate) fn strip_prefix_ignore_ascii_case<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let (pre, post) = line.split_at_checked(prefix.len())?;
    if pre.eq_ignore_ascii_case(prefix) {
        Some(post)
    } else {
        None
    }
}

mod completion;
pub use completion::Completion;

pub mod io;

#[cfg(test)]
mod test_helpers;

pub mod prelude {
    pub use crate::Completion::{self, *};
}

pub const PROTOCOL_VERSION: Version = Version::const_new("3");
