#![no_std]

#[cfg(test)]
extern crate std;

#[macro_use]
mod logger;
mod acceptor;
mod client;
mod config;
mod constants;
mod errors;
mod pubsub;
mod stderr;
mod upgrade;

pub use errors::ServerError;
pub use stderr::Stderr;

use acceptor::Acceptor;
use constants::{EVENTS, FDS, ROOT_TASK_SIZE, TASK_SIZE, TASKS, TIMERS};
use externally_driven_async_rt::Selector;
use logger::Logger;
use rustix::{
    event::{PollFd, PollFlags, poll},
    io::Errno,
};

type Context<'a> = externally_driven_async_rt::Context<
    'a,
    !,
    EVENTS,
    TASKS,
    TASK_SIZE,
    ROOT_TASK_SIZE,
    TIMERS,
    FDS,
>;
type EventLoop<'a> = externally_driven_async_rt::EventLoop<
    'a,
    !,
    EVENTS,
    TASKS,
    TASK_SIZE,
    ROOT_TASK_SIZE,
    TIMERS,
    FDS,
>;
type LoopStorage = externally_driven_async_rt::LoopStorage<
    !,
    EVENTS,
    TASKS,
    TASK_SIZE,
    ROOT_TASK_SIZE,
    TIMERS,
    FDS,
>;
type TcpListener<'a> = externally_driven_async_rt::net::TcpListener<'a, FDS>;
type TcpStream<'a> = externally_driven_async_rt::net::TcpStream<'a, FDS>;
type BorrowedReadHalf<'s, 'a> = externally_driven_async_rt::net::BorrowedReadHalf<'s, 'a, FDS>;
type BorrowedWriteHalf<'s, 'a> = externally_driven_async_rt::net::BorrowedWriteHalf<'s, 'a, FDS>;

pub fn run() -> Result<!, ServerError> {
    Logger::init()?;

    let (address, raw_address, token) = config::from_env()?;

    let mut acceptor = Acceptor::new(address, raw_address, token);

    let storage = LoopStorage::new();
    // SAFETY: `storage` outlives `event_loop`, which is dropped explicitly below, and only the
    // event loop's tasks call `wake()`, so no waker is woken after `storage` is gone.
    let storage = unsafe { core::mem::transmute::<&LoopStorage, &'static LoopStorage>(&storage) };

    let mut selector = Selector::new().map_err(ServerError::Selector)?;
    let mut event_loop = EventLoop::new(storage, &mut selector, |ctx| acceptor.run(ctx))
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
