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
#![expect(clippy::large_types_passed_by_value)]
#![expect(clippy::missing_errors_doc)]
#![expect(clippy::large_enum_variant)]
#![doc = include_str!("../README.md")]

pub use connectivity::Connectivity;
pub use mpclipboard::MPClipboard;
pub use output::Output;

pub use ffi::COutput;

mod config;
mod connection;
mod connectivity;
mod ffi;
mod logger;
mod mpclipboard;
mod output;
mod tls;
