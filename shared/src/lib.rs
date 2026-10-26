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
    message::{Message, MessageDecodeError},
    message_reader::MessageReader,
    message_writer::{MessageWriter, MessageWriterError},
};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod timerfd;

pub type Token = NonEmptyString;

pub type ID = NonEmptyString;

mod non_empty_string;
pub use non_empty_string::{NonEmptyString, NonEmptyStringError};

mod wants;
pub use wants::{OptionWantsExt, Wants};

mod polling;

mod event_loop;
pub use event_loop::{
    EventLoopFdResult, EventLoopWithTimer, EventLoopWithTimerError, EventLoopWithTimerResult,
};

mod event_loop_aware_fd;
pub use event_loop_aware_fd::EventLoopAwareFd;

mod revents;
pub use revents::{REvents, REventsError};

mod store;
pub use store::Store;

mod time;
pub use time::{Now, Past};

mod url;
pub use url::{Url, UrlParseError};

mod completion;
pub use completion::Completion;

pub mod io;

#[cfg(test)]
mod marble;

#[cfg(test)]
mod event_loop_test;

pub mod prelude {
    pub use crate::Completion::{self, *};
}

pub const PROTOCOL_VERSION: &str = "5";
