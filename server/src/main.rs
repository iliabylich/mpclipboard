#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]

use crate::{config::Config, state::State};
use anyhow::{Context as _, Result};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::net::TcpListener;

mod auth;
mod client;
mod config;
mod state;
mod store;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .write_style(env_logger::WriteStyle::Always)
        .init();

    let Config { port, token } = Config::read().await?;

    log::info!("Starting server on http://127.0.0.1:{}", port);
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .context("failed to bind")?;

    let mut state = State::new(token);
    let mut timer = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            Ok((stream, remote_addr)) = listener.accept() => {
                state.register(stream, remote_addr).await;
            }

            Some((name, clip)) = state.next() => {
                state.broadcast(name, clip).await;
            }

            _ = timer.tick() => {
                state.ping().await;
            }
        }
    }
}
