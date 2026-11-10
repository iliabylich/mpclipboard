#![no_std]

#[cfg(test)]
extern crate std;

mod acceptor;
mod client;
mod config;
mod env;
mod fixed_str;
mod handshake;
mod logger;
mod message;
mod message_reader;
mod message_stream;
mod pubsub;
mod select;
mod stderr;
mod stranger;
mod write_all;
pub use stderr::Stderr;

use acceptor::Acceptor;
use config::Config;
use core::convert::Infallible;
use externally_driven_async_rt::{
    Context, EventLoop, LoopStorage, Selector, State, net::TcpListener,
};
use logger::Logger;
use rustix::{
    event::{PollFd, PollFlags, poll},
    io::Errno,
};

const MAX_CLIENTS: usize = 10;

const MAX_TASKS_COUNT: usize = MAX_CLIENTS;
const MAX_TASK_SIZE: usize = 4896;
const MAX_ROOT_TASK_SIZE: usize = 232;
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
    type Event = Infallible;

    fn apply(&mut self, event: Self::Event) {
        match event {}
    }
}

#[expect(clippy::result_unit_err)]
pub fn run() -> Result<Infallible, ()> {
    Logger::init()?;

    let config = Config::from_env()?;
    log::info!("{config:?}");

    let listener = TcpListener::bind(config.bind_address)
        .inspect_err(|err| log::error!("failed to bind {}: {err}", config.bind_address))
        .map_err(|_| ())?;
    log::info!("listening on {}", config.bind_address);

    let mut acceptor = Acceptor::new(listener, config.token);

    let storage = ServerStorage::new();
    // SAFETY: `storage` outlives `event_loop`, which is dropped explicitly below, and only the
    // event loop's tasks call `wake()`, so no waker is woken after `storage` is gone.
    let storage =
        unsafe { core::mem::transmute::<&ServerStorage, &'static ServerStorage>(&storage) };

    let mut selector = Selector::new()
        .inspect_err(|err| log::error!("failed to create a selector: {err}"))
        .map_err(|_| ())?;
    let mut event_loop =
        ServerLoop::new(storage, &mut selector, DummyState, |ctx| acceptor.run(ctx))
            .inspect_err(|err| log::error!("failed to start the event loop: {err}"))
            .map_err(|_| ())?;

    loop {
        match poll(&mut [PollFd::new(&event_loop, PollFlags::IN)], None) {
            Ok(_) | Err(Errno::INTR) => {}
            Err(err) => {
                log::error!("poll() failed: {err}");
                break;
            }
        }
        if let Err(err) = event_loop.drain_events_without_blocking() {
            log::error!("{err}");
            break;
        }
    }
    drop(event_loop);
    Err(())
}
