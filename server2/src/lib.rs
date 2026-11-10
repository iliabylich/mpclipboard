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
mod fixed_str;
mod handshake;
mod id;
mod message;
mod message_reader;
mod message_stream;
mod pubsub;
mod select;
mod stderr;
mod stranger;
mod token;
mod write_all;

pub use stderr::Stderr;

use acceptor::Acceptor;
use constants::{IO_COUNT, ROOT_TASK_SIZE, TASK_COUNT, TASK_SIZE, TIMER_COUNT};
use core::convert::Infallible;
use externally_driven_async_rt::{Context, EventLoop, LoopStorage, Selector, net::TcpListener};
use logger::Logger;
use rustix::{
    event::{PollFd, PollFlags, poll},
    io::Errno,
};

type ServerContext<'a> =
    Context<'a, (), TASK_COUNT, TASK_SIZE, ROOT_TASK_SIZE, TIMER_COUNT, IO_COUNT>;
type ServerLoop<'a> =
    EventLoop<'a, (), TASK_COUNT, TASK_SIZE, ROOT_TASK_SIZE, TIMER_COUNT, IO_COUNT>;
type ServerStorage = LoopStorage<(), TASK_COUNT, TASK_SIZE, ROOT_TASK_SIZE, TIMER_COUNT, IO_COUNT>;

#[expect(clippy::result_unit_err)]
pub fn run() -> Result<Infallible, ()> {
    Logger::init()?;

    let (address, token) = config::from_env()?;

    info!("Trying to bind on {address}");
    let listener = TcpListener::bind(address)
        .inspect_err(|err| error!("failed to bind {}: {err}", address))
        .map_err(|_| ())?;
    info!("listening on {}", address);

    let mut acceptor = Acceptor::new(listener, token);

    let storage = ServerStorage::new();
    // SAFETY: `storage` outlives `event_loop`, which is dropped explicitly below, and only the
    // event loop's tasks call `wake()`, so no waker is woken after `storage` is gone.
    let storage =
        unsafe { core::mem::transmute::<&ServerStorage, &'static ServerStorage>(&storage) };

    let mut selector = Selector::new()
        .inspect_err(|err| error!("failed to create a selector: {err}"))
        .map_err(|_| ())?;
    let mut event_loop = ServerLoop::new(storage, &mut selector, (), |ctx| acceptor.run(ctx))
        .inspect_err(|err| error!("failed to start the event loop: {err}"))
        .map_err(|_| ())?;

    loop {
        match poll(&mut [PollFd::new(&event_loop, PollFlags::IN)], None) {
            Ok(_) | Err(Errno::INTR) => {}
            Err(err) => {
                error!("poll() failed: {err}");
                break;
            }
        }
        if let Err(err) = event_loop.drain_events_without_blocking() {
            error!("{err}");
            break;
        }
    }
    drop(event_loop);
    Err(())
}
