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
use externally_driven_async_rt::{
    Context, EventLoop, LoopStorage, Selector, State, net::TcpListener,
};
use logger::Logger;
use message::Message;
use request_reader::Token;
use rustix::{
    event::{PollFd, PollFlags, poll},
    io::Errno,
};
use slot::Slot;

const MAX_CLIENTS: usize = 20;

const MAX_TASKS_COUNT: usize = MAX_CLIENTS;
const MAX_TASK_SIZE: usize = 2856;
const MAX_ROOT_TASK_SIZE: usize = 544;
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
type ServerLoop<'a> = EventLoop<
    'a,
    DummyState,
    MAX_TASKS_COUNT,
    MAX_TASK_SIZE,
    MAX_ROOT_TASK_SIZE,
    MAX_TIMERS_COUNT,
    MAX_IO_COUNT,
>;
type ServerStorage = LoopStorage<
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

fn start<'a>(
    selector: &'a mut Selector,
    listener: TcpListener,
    token: &'a Token,
    slots: &'a [RefCell<Slot>],
    current: &'a Cell<Option<Message>>,
) -> Result<ServerLoop<'a>, externally_driven_async_rt::Error> {
    static STORAGE: ServerStorage = ServerStorage::new();
    ServerLoop::new(
        &STORAGE,
        selector,
        DummyState,
        async move |ctx: &ServerContext<'a>| {
            accept_loop(ctx, listener, token, slots, current).await;
        },
    )
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

    let listener = match TcpListener::bind(config.bind_address) {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("failed to bind {}: {err}", config.bind_address);
            return 1;
        }
    };
    log::info!("listening on {}", config.bind_address);

    let token = config.token;
    let current = Cell::new(None);
    let slots = [const { RefCell::new(Slot::Free) }; MAX_CLIENTS];

    let mut selector = match Selector::new() {
        Ok(selector) => selector,
        Err(err) => {
            log::error!("failed to create a selector: {err}");
            return 1;
        }
    };
    let mut event_loop = match start(&mut selector, listener, &token, &slots, &current) {
        Ok(event_loop) => event_loop,
        Err(err) => {
            log::error!("failed to start the event loop: {err}");
            return 1;
        }
    };

    loop {
        match poll(&mut [PollFd::new(&event_loop, PollFlags::IN)], None) {
            Ok(_) | Err(Errno::INTR) => {}
            Err(err) => {
                log::error!("poll() failed: {err}");
                return 1;
            }
        }
        if let Err(err) = event_loop.drain_events_without_blocking() {
            log::error!("{err}");
            return 1;
        }
    }
}
