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
#![allow(clippy::future_not_send)]
#![doc = include_str!("../README.md")]

use anyhow::Result;
use main_loop::MainLoop;
use mpclipboard::MPClipboardStream;

mod clipboard;
mod main_loop;
mod mpclipboard;
mod tray;

#[tokio::main]
async fn main() -> Result<()> {
    MPClipboardStream::init()?;
    let main_loop = MainLoop::new().await?;
    main_loop.start().await?;
    Ok(())
}
