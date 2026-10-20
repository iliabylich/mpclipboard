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

extern crate alloc;

mod config;
pub use config::{
    ConfigIdError, ConfigReader, ConfigReaderError, ConfigTokenError, ConfigUrlError,
};

mod array_writer;

mod request;
mod request_reader;
pub use self::{
    request::Request,
    request_reader::{RequestReader, RequestReaderError},
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

mod non_empty_inline_string;
pub use non_empty_inline_string::{NonEmptyInlineString, NonEmptyInlineStringError};

mod buffer;
pub use buffer::{Buffer, PushResult};

mod wants;
pub use wants::{OptionWantsExt, Wants};

mod event_loop;
pub use event_loop::{
    EventLoopFdResult, EventLoopWithTimer, EventLoopWithTimerError, EventLoopWithTimerResult,
    RegisteredFd,
};

mod revents;
pub use revents::{REvents, REventsError};

mod store;
pub use store::Store;

mod url;
pub use url::{Url, UrlParseError};

mod completion;
pub use completion::Completion;

pub mod io;

#[cfg(test)]
mod test_helpers;

pub mod prelude {
    pub use crate::Completion::{self, *};
}

pub const PROTOCOL_VERSION: Version = Version::const_new("5");
