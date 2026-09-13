// #![no_std]
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
#![doc = include_str!("../README.md")]

mod config;
pub use config::{ConfigParser, ConfigParserError};

mod http_lines_buffer;
pub use http_lines_buffer::{HttpLinesBufferOverflowError, HttpLinesBufferUnderflowError};
mod http_lines_reader;
pub use http_lines_reader::HttpLinesReaderError;

mod writebuf;
mod writer;

mod array_writer;

mod upgrade_request;
pub use upgrade_request::UpgradeRequest;

mod upgrade_request_reader;
pub use upgrade_request_reader::{UpgradeRequestParserError, UpgradeRequestReader};

mod upgrade_request_writer;
pub use upgrade_request_writer::UpgradeRequestWriter;

mod upgrade_response;

mod upgrade_response_reader;
pub use upgrade_response_reader::{UpgradeResponseParserError, UpgradeResponseReader};

mod upgrade_response_writer;
pub use upgrade_response_writer::UpgradeResponseWriter;

mod message;
pub use message::{Message, MessageDecodeError};

mod message_reader;
pub use message_reader::MessageReader;

mod message_writer;
pub use message_writer::MessageWriter;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod timerfd;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use timerfd::Timerfd;

pub(crate) const MAX_HOST_LENGTH: usize = 249;
pub(crate) const MAX_PORT_LENGTH: usize = 5;
pub(crate) const MAX_HOST_PORT_LENGTH: usize = MAX_HOST_LENGTH + 1 + MAX_PORT_LENGTH;
const _: () = assert!(MAX_HOST_PORT_LENGTH == 255);

pub type HostPort = NonEmptyInlineString<MAX_HOST_PORT_LENGTH>;

pub(crate) const MAX_TOKEN_LENGTH: usize = 100;
pub type Token = NonEmptyInlineString<MAX_TOKEN_LENGTH>;

pub(crate) const MAX_ID_LENGTH: usize = 100;
pub type ID = NonEmptyInlineString<MAX_ID_LENGTH>;

pub(crate) const START_LINE: &str = "GET / HTTP/1.1";
pub(crate) const HOST_PREFIX: &str = "Host: ";
pub(crate) const TOKEN_PREFIX: &str = "Token: ";
pub(crate) const ID_PREFIX: &str = "ID: ";
pub(crate) const CONNECTION_UPGRADE_HEADER: &str = "Connection: Upgrade";
pub(crate) const UPGRADE_MPCLIPBOARD_RAW_HEADER: &str = "Upgrade: mpclipboard-raw";
pub(crate) const PADDING_PREFIX: &str = "Padding: ";
pub(crate) const MIN_PADDING_LENGTH: usize = 1;

mod non_empty_inline_string;
pub use non_empty_inline_string::NonEmptyInlineString;

mod wants;
pub use wants::Wants;

mod event_loop;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use event_loop::EpollError;
pub use event_loop::{EventLoop, EventLoopResult};

#[doc(hidden)]
pub mod logger;

mod revents;
pub use revents::{REvents, REventsError};

mod store;
pub use store::Store;

mod tcp_keep_alive;
pub use tcp_keep_alive::enable_tcp_keep_alive;

mod url;
pub use url::{Url, UrlError};

pub(crate) fn strip_prefix_ignore_ascii_case<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let (pre, post) = line.split_at_checked(prefix.len())?;
    if pre.eq_ignore_ascii_case(prefix) {
        Some(post)
    } else {
        None
    }
}
