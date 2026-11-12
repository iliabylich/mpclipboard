use crate::{
    Connectivity,
    config::Config,
    connection::Connection,
    constants::{FDS, FREEZE_TIME, ROOT_TASK_SIZE, TASK_SIZE, TASKS, TIMERS},
    loop_state::{LoopEvent, LoopState},
    outbox::Outbox,
};
use core::{
    cell::{RefCell, RefMut},
    mem::ManuallyDrop,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};
use externally_driven_async_rt::{Context, EventLoop, LoopStorage, Selector};
use mpclipboard_shared::Message;
use rustix::{
    fd::{AsFd, BorrowedFd},
    io::Errno,
};
use rustls::ClientConfig;
use std::sync::Arc;

pub(crate) type LoopContext<'a> =
    Context<'a, LoopState, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>;
type Loop = EventLoop<'static, LoopState, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>;
type Storage = LoopStorage<TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>;

static STORAGE: Storage = Storage::new();
static CREATED: AtomicBool = AtomicBool::new(false);

pub(crate) struct Runtime {
    event_loop: ManuallyDrop<Loop>,
    data: NonNull<Data>,
}

impl Runtime {
    pub(crate) fn new(config: Config, tls_config: Arc<ClientConfig>) -> Result<Self, RuntimeError> {
        if CREATED.swap(true, Ordering::Relaxed) {
            return Err(RuntimeError::AlreadyCreated);
        }

        let selector = Selector::new().map_err(RuntimeError::Selector)?;
        let (data, selector, state, outbox, config, tls_config) =
            Data::new(selector, config, tls_config);

        let result = Loop::new(&STORAGE, selector, state, |ctx| async move {
            let connection = Connection::new(ctx, config, tls_config, outbox);

            loop {
                ctx.dispatch(LoopEvent::Connectivity(Connectivity::Connecting));

                let Err(err) = connection.run().await;
                log::error!("{err}");

                ctx.dispatch(LoopEvent::Connectivity(Connectivity::Disconnected));
                ctx.sleep(FREEZE_TIME).await;
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

    pub(crate) fn drain(&mut self) -> Result<(), RuntimeError> {
        self.event_loop
            .drain_events_without_blocking()
            .map_err(RuntimeError::Drain)
    }

    pub(crate) fn state_mut(&mut self) -> RefMut<'_, LoopState> {
        self.event_loop.state_mut()
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
    state: RefCell<LoopState>,
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
        &'static mut RefCell<LoopState>,
        &'static Outbox,
        &'static Config,
        &'static Arc<ClientConfig>,
    ) {
        let data = NonNull::from(Box::leak(Box::new(Self {
            selector,
            state: RefCell::new(LoopState::new()),
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
                &mut (*ptr).state,
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
    Start(externally_driven_async_rt::Error),
    #[error("failed to drain event loop: {0}")]
    Drain(externally_driven_async_rt::Error),
    #[error("failed to notify event loop: {0}")]
    Notify(Errno),
}
