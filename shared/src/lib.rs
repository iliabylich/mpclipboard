#![no_std]
#![forbid(unsafe_code)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications,
    deprecated_in_future,
    unused_lifetimes,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::pedantic,
    clippy::nursery,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]
#![expect(clippy::missing_errors_doc, clippy::redundant_pub_crate)]
#![allow(clippy::option_if_let_else)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![doc = include_str!("../README.md")]

#[cfg(test)]
extern crate alloc;

use generic_array::GenericArray;
use typenum::{U1, U2, U4, U5, U6, U7, U9, U14, U19, U24, U50, U100, U249, Unsigned, op};

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

pub(crate) type MaxHostLength = U249;
type MaxPortLength = U5;
pub(crate) type MaxHostPortLength = op!(MaxHostLength + U1 + MaxPortLength);
const _: () = assert!(MaxHostPortLength::USIZE == 255);

pub type HostPort = NonEmptyInlineString<MaxHostPortLength>;

pub(crate) type MaxTokenLength = U100;
pub type Token = NonEmptyInlineString<MaxTokenLength>;

pub(crate) type MaxIdLength = U100;
pub type ID = NonEmptyInlineString<MaxIdLength>;

pub(crate) type MaxVersionLength = U50;
pub type Version = NonEmptyInlineString<MaxVersionLength>;

pub(crate) type StartLineLength = U14;
pub(crate) const START_LINE: GenericArray<u8, StartLineLength> =
    GenericArray::from_array(*b"GET / HTTP/1.1");

pub(crate) type HostPrefixLength = U6;
pub(crate) const HOST_PREFIX: GenericArray<u8, HostPrefixLength> =
    GenericArray::from_array(*b"Host: ");

pub(crate) type TokenPrefixLength = U7;
pub(crate) const TOKEN_PREFIX: GenericArray<u8, TokenPrefixLength> =
    GenericArray::from_array(*b"Token: ");

pub(crate) type IdPrefixLength = U4;
pub(crate) const ID_PREFIX: GenericArray<u8, IdPrefixLength> = GenericArray::from_array(*b"ID: ");

pub(crate) type VersionPrefixLength = U9;
pub(crate) const VERSION_PREFIX: GenericArray<u8, VersionPrefixLength> =
    GenericArray::from_array(*b"Version: ");

pub(crate) type ConnectionUpgradeHeaderLength = U19;
pub(crate) const CONNECTION_UPGRADE_HEADER: GenericArray<u8, ConnectionUpgradeHeaderLength> =
    GenericArray::from_array(*b"Connection: Upgrade");

pub(crate) type UpgradeMpclipboardRawHeaderLength = U24;
pub(crate) const UPGRADE_MPCLIPBOARD_RAW_HEADER: GenericArray<
    u8,
    UpgradeMpclipboardRawHeaderLength,
> = GenericArray::from_array(*b"Upgrade: mpclipboard-raw");

pub(crate) type CrlfLength = U2;
pub(crate) const CRLF: GenericArray<u8, CrlfLength> = GenericArray::from_array(*b"\r\n");

mod non_empty_inline_string;
pub use non_empty_inline_string::{
    NonEmptyInlineString, NonEmptyInlineStringError, NonEmptyInlineStringLength,
};

mod buffer;
pub use buffer::{Buffer, PushResult};

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

pub(crate) fn strip_prefix_ignore_ascii_case<'a>(line: &'a str, prefix: &[u8]) -> Option<&'a str> {
    let (pre, post) = line.split_at_checked(prefix.len())?;
    if pre.as_bytes().eq_ignore_ascii_case(prefix) {
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
