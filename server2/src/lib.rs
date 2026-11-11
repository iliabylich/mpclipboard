#![no_std]

#[cfg(test)]
extern crate std;

#[macro_use]
mod logger;
mod acceptor;
mod client;
mod config;
mod constants;
mod env;
mod errors;
mod fixed_str;
mod handshake;
mod id;
mod message;
mod message_reader;
mod message_stream;
mod pubsub;
mod stderr;
mod stranger;
mod token;

pub use errors::ServerError;
pub use stderr::Stderr;

use acceptor::Acceptor;
use constants::{FDS, ROOT_TASK_SIZE, TASK_SIZE, TASKS, TIMERS};
use core::{cell::RefCell, convert::Infallible};
use externally_driven_async_rt::{Context, EventLoop, LoopStorage, Selector, net::TcpListener};
use logger::Logger;
use rustix::{
    event::{PollFd, PollFlags, poll},
    io::Errno,
};

type ServerContext<'a> = Context<'a, (), TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>;
type ServerLoop<'a> = EventLoop<'a, (), TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>;
type ServerStorage = LoopStorage<TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>;

pub fn run() -> Result<Infallible, ServerError> {
    Logger::init()?;

    let (address, raw_address, token) = config::from_env()?;

    info!("Trying to bind on {raw_address}");
    let listener = TcpListener::bind(address).map_err(|err| ServerError::Bind {
        address: raw_address,
        err,
    })?;
    info!("listening on {raw_address}");

    let mut acceptor = Acceptor::new(listener, token);

    let storage = ServerStorage::new();
    // SAFETY: `storage` outlives `event_loop`, which is dropped explicitly below, and only the
    // event loop's tasks call `wake()`, so no waker is woken after `storage` is gone.
    let storage =
        unsafe { core::mem::transmute::<&ServerStorage, &'static ServerStorage>(&storage) };

    let mut selector = Selector::new().map_err(ServerError::Selector)?;
    let mut state = RefCell::new(());
    let mut event_loop =
        ServerLoop::new(storage, &mut selector, &mut state, |ctx| acceptor.run(ctx))
            .map_err(ServerError::EventLoopStart)?;

    let err = loop {
        match poll(&mut [PollFd::new(&event_loop, PollFlags::IN)], None) {
            Ok(_) | Err(Errno::INTR) => {}
            Err(err) => break ServerError::Poll(err),
        }
        if let Err(err) = event_loop.drain_events_without_blocking() {
            break ServerError::EventLoop(err);
        }
    };
    drop(event_loop);
    Err(err)
}
