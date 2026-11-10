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

use client::ClientState;
use config::Config;
use core::cell::{Cell, RefCell, RefMut};
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

const MAX_CLIENTS: usize = 10;

const MAX_TASKS_COUNT: usize = MAX_CLIENTS;
const MAX_TASK_SIZE: usize = 576;
const MAX_ROOT_TASK_SIZE: usize = 328;
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
    storage: &'static ServerStorage,
    selector: &'a mut Selector,
    listener: TcpListener,
    token: &'a Token,
    slots: &'a [RefCell<Slot>],
    current: &'a Cell<Option<Message>>,
    states: &'a [RefCell<Option<ClientState>>],
) -> Result<ServerLoop<'a>, externally_driven_async_rt::Error> {
    ServerLoop::new(
        storage,
        selector,
        DummyState,
        move |ctx: ServerContext<'a>| accept_loop(ctx, listener, token, slots, current, states),
    )
}

async fn accept_loop<'a>(
    ctx: ServerContext<'a>,
    mut listener: TcpListener,
    token: &'a Token,
    slots: &'a [RefCell<Slot>],
    current: &'a Cell<Option<Message>>,
    states: &'a [RefCell<Option<ClientState>>],
) {
    loop {
        let task = ctx.reserve_task().await;
        let (stream, peer) = loop {
            match listener.accept(&ctx).await {
                Ok(accepted) => break accepted,
                Err(err) => log::error!("accept() failed: {err}"),
            }
        };
        log::trace!("[{peer}] accepted");
        let Some(state) = states.iter().find_map(|state| state.try_borrow_mut().ok()) else {
            log::error!("[{peer}] no free client state");
            continue;
        };
        let state = RefMut::map(state, |state| state.insert(ClientState::new(peer)));
        task.spawn(move |ctx| client::client(ctx, stream, token, slots, current, state));
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
    let states = [const { RefCell::new(None) }; MAX_CLIENTS];

    let storage = ServerStorage::new();
    // SAFETY: `storage` outlives `event_loop`, which is dropped explicitly below, and only the
    // event loop's tasks call `wake()`, so no waker is woken after `storage` is gone.
    let storage =
        unsafe { core::mem::transmute::<&ServerStorage, &'static ServerStorage>(&storage) };

    let mut selector = match Selector::new() {
        Ok(selector) => selector,
        Err(err) => {
            log::error!("failed to create a selector: {err}");
            return 1;
        }
    };
    let mut event_loop = match start(
        storage,
        &mut selector,
        listener,
        &token,
        &slots,
        &current,
        &states,
    ) {
        Ok(event_loop) => event_loop,
        Err(err) => {
            log::error!("failed to start the event loop: {err}");
            return 1;
        }
    };

    let code = loop {
        match poll(&mut [PollFd::new(&event_loop, PollFlags::IN)], None) {
            Ok(_) | Err(Errno::INTR) => {}
            Err(err) => {
                log::error!("poll() failed: {err}");
                break 1;
            }
        }
        if let Err(err) = event_loop.drain_events_without_blocking() {
            log::error!("{err}");
            break 1;
        }
    };
    drop(event_loop);
    code
}
