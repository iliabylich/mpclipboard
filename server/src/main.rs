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
#![doc = include_str!("../README.md")]

use anyhow::Result;

mod config;
use config::Config;

mod as_poll_fd;
mod client;
mod fd_set;
mod pre_sink;
mod pre_source;
mod reaper;
mod tcp_listener;
mod url_ext;

mod main_loop;
use main_loop::MainLoop;

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::read()?;

    let mut main_loop = MainLoop::new(&config)?;

    loop {
        main_loop.poll_and_process_events();
    }
}
