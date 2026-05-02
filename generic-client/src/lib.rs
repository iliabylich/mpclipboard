#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::redundant_pub_crate)]
#![doc = include_str!("../README.md")]

pub use config::Config;
pub use config::ConfigReadOption;
pub use connectivity::Connectivity;
pub use context::Context;
pub use mpclipboard::MPClipboard;
pub use output::Output;

#[cfg(target_os = "android")]
pub use ffi::mpclipboard_setup_rustls_on_jvm;
pub use ffi::{COutput, PushResult};

mod config;
mod connectivity;
mod context;
mod event_loop;
mod ffi;
mod logger;
mod mpclipboard;
mod output;
mod state;
mod timer;
mod tls;
