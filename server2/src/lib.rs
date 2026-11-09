#![no_std]

#[cfg(test)]
extern crate std;

mod client;
mod config;
mod env;
mod fixed_str;
mod logger;
mod message;
mod message_reader;
mod request_reader;
mod select;
mod slot;
mod stderr;
pub use stderr::Stderr;

use config::Config;
use core::cell::{Cell, RefCell};
use externally_driven_async_rt::{Context, Selector, State, net::TcpListener};
use logger::Logger;
use message::Message;
use request_reader::Token;
use slot::Slot;

const MAX_CLIENTS: usize = 20;

const MAX_TASKS_COUNT: usize = MAX_CLIENTS;
const MAX_TASK_SIZE: usize = 4096;
const MAX_ROOT_TASK_SIZE: usize = 1024;
const MAX_TIMERS_COUNT: usize = MAX_CLIENTS;
const MAX_IO_COUNT: usize = MAX_CLIENTS + 1;

type ServerContext<'a> = Context<
    'a,
    DummyState,
    MAX_TASKS_COUNT,
    MAX_TASK_SIZE,
    MAX_ROOT_TASK_SIZE,
    MAX_TIMERS_COUNT,
    MAX_IO_COUNT,
>;

struct DummyState;

impl State for DummyState {
    type Event = core::convert::Infallible;

    fn apply(&mut self, event: Self::Event) {
        match event {}
    }
}

async fn accept_loop<'a>(
    ctx: &ServerContext<'a>,
    mut listener: TcpListener,
    token: &'a Token,
    slots: &'a [RefCell<Slot>],
    current: &'a Cell<Option<Message>>,
) {
    loop {
        let task = ctx.reserve_task().await;
        let (stream, peer) = loop {
            match listener.accept(ctx).await {
                Ok(accepted) => break accepted,
                Err(err) => log::error!("accept() failed: {err}"),
            }
        };
        log::trace!("[{peer}] accepted");
        task.spawn(async move |ctx| {
            client::client(ctx, stream, peer, token, slots, current).await;
        });
    }
}

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
