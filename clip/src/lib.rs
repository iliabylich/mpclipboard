#![warn(missing_docs)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![doc = include_str!("../README.md")]

mod error;
pub use error::ParseClipError;

mod text_or_binary;
pub use text_or_binary::TextOrBinary;

mod clip;
pub use clip::Clip;
