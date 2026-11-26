use crate::{
    Connectivity,
    config::Config,
    connection::Connection,
    constants::{EVENTS, FDS, FREEZE_TIME, ROOT_TASK_SIZE, TASK_SIZE, TASKS, TIMERS},
    loop_state::{LoopEvent, LoopState},
    outbox::Outbox,
};
use alloc::sync::Arc;
use core::{
    mem::ManuallyDrop,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};
use externally_driven_async_rt::{EventLoopError, Selector};
use mpclipboard_shared::Message;
use rustix::{
    fd::{AsFd, BorrowedFd},
    io::Errno,
};
use rustls::ClientConfig;

pub(crate) type Context<'a> = externally_driven_async_rt::Context<
    'a,
    LoopEvent,
    EVENTS,
    TASKS,
    TASK_SIZE,
    ROOT_TASK_SIZE,
    TIMERS,
    FDS,
>;
type EventLoop = externally_driven_async_rt::EventLoop<
    'static,
    LoopEvent,
    EVENTS,
    TASKS,
    TASK_SIZE,
    ROOT_TASK_SIZE,
    TIMERS,
    FDS,
>;
type LoopStorage = externally_driven_async_rt::LoopStorage<
    LoopEvent,
    EVENTS,
    TASKS,
    TASK_SIZE,
    ROOT_TASK_SIZE,
    TIMERS,
    FDS,
>;
pub(crate) type TcpStream = externally_driven_async_rt::net::TcpStream<'static, FDS>;
pub(crate) type UdpSocket = externally_driven_async_rt::net::UdpSocket<'static, FDS>;
pub(crate) type BorrowedReadHalf<'s> =
    externally_driven_async_rt::net::BorrowedReadHalf<'s, 'static, FDS>;
pub(crate) type BorrowedWriteHalf<'s> =
    externally_driven_async_rt::net::BorrowedWriteHalf<'s, 'static, FDS>;

static STORAGE: LoopStorage = LoopStorage::new();
static CREATED: AtomicBool = AtomicBool::new(false);

pub(crate) struct Runtime {
    event_loop: ManuallyDrop<EventLoop>,
    data: NonNull<Data>,
}

impl Runtime {
    pub(crate) fn new(config: Config, tls_config: Arc<ClientConfig>) -> Result<Self, RuntimeError> {
        if CREATED.swap(true, Ordering::Relaxed) {
            return Err(RuntimeError::AlreadyCreated);
        }

        let selector = Selector::new().map_err(RuntimeError::Selector)?;
        let (data, selector, outbox, config, tls_config) = Data::new(selector, config, tls_config);

        let result = EventLoop::new(&STORAGE, selector, |ctx| async move {
            let connection = Connection::new(ctx, config, tls_config, outbox);

            loop {
                if let Err(event) = ctx.dispatch(LoopEvent::Connectivity(Connectivity::Connecting))
                {
                    log::warn!("events queue is full, dropping {event:?}");
                }

                let Err(err) = connection.run().await;
                log::error!("{err}");

                if let Err(event) =
                    ctx.dispatch(LoopEvent::Connectivity(Connectivity::Disconnected))
                {
                    log::warn!("events queue is full, dropping {event:?}");
                }
                ctx.sleep(FREEZE_TIME)
                    .await
                    .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));
            }
        });

        match result {
            Ok(event_loop) => Ok(Self {
                event_loop: ManuallyDrop::new(event_loop),
                data,
            }),
            Err(err) => {
                // SAFETY: the event loop that borrowed `data` is already gone.
                drop(unsafe { Box::from_raw(data.as_ptr()) });
                Err(RuntimeError::Start(err))
            }
        }
    }

    pub(crate) fn drain(
        &mut self,
    ) -> Result<(Option<Connectivity>, Option<Message>), RuntimeError> {
        self.event_loop
            .drain_events_without_blocking()
            .map_err(RuntimeError::Drain)?;
        let mut state = LoopState::new();
        for event in self.event_loop.events() {
            state.apply(event);
        }
        Ok(state.take_updates())
    }

    pub(crate) fn push(&self, message: Message) -> Result<(), RuntimeError> {
        // SAFETY: see `Runtime::new`, the outbox is only ever borrowed immutably.
        let outbox = unsafe { &(*self.data.as_ptr()).outbox };
        outbox.put(message);
        self.event_loop.notify().map_err(RuntimeError::Notify)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // SAFETY: the event loop is dropped exactly once, before the `data` it borrows,
        // which are freed exactly once.
        unsafe {
            ManuallyDrop::drop(&mut self.event_loop);
            drop(Box::from_raw(self.data.as_ptr()));
        }
    }
}

impl AsFd for Runtime {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.event_loop.as_fd()
    }
}

struct Data {
    selector: Selector,
    outbox: Outbox,
    config: Config,
    tls_config: Arc<ClientConfig>,
}

impl Data {
    fn new(
        selector: Selector,
        config: Config,
        tls_config: Arc<ClientConfig>,
    ) -> (
        NonNull<Self>,
        &'static mut Selector,
        &'static Outbox,
        &'static Config,
        &'static Arc<ClientConfig>,
    ) {
        let data = NonNull::from(Box::leak(Box::new(Self {
            selector,
            outbox: Outbox::default(),
            config,
            tls_config,
        })));

        // SAFETY: `data` is a leaked heap allocation, so it lives until it's freed through the
        // returned pointer, which `Runtime` does only in `Drop`, after the event loop (the only
        // holder of these references) is dropped. The references point to disjoint fields, and
        // `Runtime` never creates a reference to the whole `Data`.
        unsafe {
            let ptr = data.as_ptr();
            (
                data,
                &mut (*ptr).selector,
                &(*ptr).outbox,
                &(*ptr).config,
                &(*ptr).tls_config,
            )
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("MPClipboard can be created only once per process")]
    AlreadyCreated,
    #[error("failed to create selector: {0}")]
    Selector(Errno),
    #[error("failed to start event loop: {0}")]
    Start(EventLoopError),
    #[error("failed to drain event loop: {0}")]
    Drain(EventLoopError),
    #[error("failed to notify event loop: {0}")]
    Notify(Errno),
}
