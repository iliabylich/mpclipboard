#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications,
    deprecated_in_future,
    unused_lifetimes,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::pedantic,
    clippy::nursery,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]
#![expect(
    clippy::large_types_passed_by_value,
    clippy::missing_errors_doc,
    clippy::large_enum_variant,
    clippy::redundant_pub_crate
)]
#![allow(clippy::option_if_let_else)]

extern crate alloc;

pub use config::ConfigError;
pub use connectivity::Connectivity;
pub use mpclipboard::{MPClipboard, MPClipboardError};
pub use mpclipboard_shared::EventLoopError;
pub use output::Output;

mod config;
mod connection;
mod connectivity;
mod ffi;
mod logger;
mod mpclipboard;
mod output;
mod tls;
