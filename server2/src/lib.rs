#![no_std]

#[cfg(test)]
extern crate std;

mod config;
mod env;
mod fixed_str;
mod logger;
mod message;
mod message_reader;
mod request_reader;
mod slot;
mod stderr;
pub use stderr::Stderr;

use config::Config;
use externally_driven_async_rt::Selector;
use logger::Logger;

pub fn run() -> i32 {
    if Logger::init().is_err() {
        return 1;
    }

    let Ok(config) = Config::from_env() else {
        return 1;
    };
    log::info!("{config:?}");

    match Selector::new() {
        Ok(_) => 0,
        Err(err) => {
            log::error!("failed to create a selector: {err}");
            1
        }
    }
}
