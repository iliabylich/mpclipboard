#![warn(missing_docs)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![doc = include_str!("../README.md")]

pub use config::Config;
pub use config::ConfigReadOption;
pub use connectivity::Connectivity;
pub use context::Context;
pub use mpclipboard::MPClipboard;
pub use output::Output;

pub use ffi::COutput;
#[cfg(target_os = "android")]
pub use ffi::mpclipboard_setup_rustls_on_jvm;

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
