use anyhow::{Context as _, Result};
use os_pipe::PipeReader;
use std::io::Read;
use std::{io::ErrorKind, os::fd::AsFd};
use tokio::{
    io::unix::AsyncFd,
    sync::mpsc::{Receiver, channel},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy,
    backend::WaylandError,
    event_created_child,
    protocol::{wl_registry::WlRegistry, wl_seat::WlSeat},
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
    zwlr_data_control_source_v1::ZwlrDataControlSourceV1,
};

pub(crate) struct LocalReader {
    rx: Receiver<String>,
    handle: JoinHandle<()>,
}

impl LocalReader {
    pub(crate) async fn spawn(token: CancellationToken) -> Self {
        let (tx, rx) = channel(255);
        let handle = tokio::spawn(async move {
            let (mut state, mut queue) = match State::new().await {
                Ok(ok) => ok,
                Err(err) => {
                    log::error!(target: "LocalReader", "{err:?}");
                    return;
                }
            };

            loop {
                tokio::select! {
                    text = state.recv(&mut queue) => {
                        match text {
                            Ok(text) => {
                                if tx.send(text).await.is_err() {
                                    log::error!(target: "LocalReader", "failed to send text back: channel is closed");
                                    break;
                                }
                            }
                            Err(err) => {
                                log::error!(target: "LocalReader", "{err:?}");
                                break;
                            }
                        }
                    }

                    _ = token.cancelled() => {
                        log::info!(target: "LocalReader", "exiting...");
                        break;
                    }
                }
            }
        });

        Self { rx, handle }
    }

    pub(crate) async fn recv(&mut self) -> Option<String> {
        self.rx.recv().await
    }

    pub(crate) async fn wait(self) -> Result<()> {
        self.handle
            .await
            .context("failed to await local clipboard task")
    }
}

pub(crate) struct State {
    seat: Option<WlSeat>,
    got_seat_name: bool,
    data_manager: Option<ZwlrDataControlManagerV1>,
    conn: Connection,
    reader: Option<PipeReader>,
}

impl State {
    pub(crate) async fn new() -> Result<(Self, EventQueue<Self>)> {
        let conn =
            Connection::connect_to_env().context("failed to connect to Wayland compositor")?;

        let mut queue = conn.new_event_queue();
        let queue_handle = queue.handle();
        let display = conn.display();
        display.get_registry(&queue_handle, ());

        let mut this = Self {
            seat: None,
            got_seat_name: false,
            data_manager: None,
            conn,
            reader: None,
        };

        this.dispatch(&mut queue)
            .await
            .context("failed to dispatch")?;

        let seat = this.seat.take().context("failed to acquire seat")?;
        let data_manager = this
            .data_manager
            .take()
            .context("failed to acquire data manager")?;

        while !this.got_seat_name {
            queue.roundtrip(&mut this).context("roundtrip failed")?;
        }

        data_manager.get_data_device(&seat, &queue_handle, ());

        Ok((this, queue))
    }

    // async equivalent of EventQueue::block_dispatch
    async fn dispatch(&mut self, queue: &mut EventQueue<Self>) -> Result<usize> {
        let dispatched = queue
            .dispatch_pending(self)
            .context("failed to dispatch_pending")?;

        if dispatched > 0 {
            return Ok(dispatched);
        }

        self.conn.flush().context("failed to flush conn")?;
        if let Some(guard) = self.conn.prepare_read() {
            let fd = guard.connection_fd();
            let async_fd = AsyncFd::new(fd).context("failed to construct AsyncFd")?;
            let _ = async_fd
                .readable()
                .await
                .context("failed to wait for fd to become readable")?;
            drop(async_fd);
            let _ = fd;

            match guard.read() {
                Ok(n) => Ok(n),
                // if we are still "wouldblock", just return 0; the caller will retry.
                Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(0),
                Err(e) => Err(e),
            }?;
        }
        queue
            .dispatch_pending(self)
            .context("failed to dispatch pending")
    }

    async fn recv(&mut self, queue: &mut EventQueue<Self>) -> Result<String> {
        loop {
            if let Some(text) = self.try_recv(queue).await? {
                return Ok(text);
            }
        }
    }

    async fn try_recv(&mut self, queue: &mut EventQueue<Self>) -> Result<Option<String>> {
        self.dispatch(queue).await.context("failed to dispatch")?;

        if let Some(mut reader) = self.reader.take() {
            queue.roundtrip(self).context("failed to roundtrip")?;

            let mut buf = vec![];
            let len = reader
                .read_to_end(&mut buf)
                .context("failed to read from pipe")?;
            if len > 0
                && let Ok(string) = String::from_utf8(buf)
            {
                return Ok(Some(string));
            };
        }

        Ok(None)
    }
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_registry::Event;
        if let Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == WlSeat::interface().name {
                log::error!(target: "LocalReader", "got seat");
                state.seat = Some(registry.bind(name, version, qh, ()));
            }

            if interface == ZwlrDataControlManagerV1::interface().name {
                log::error!(target: "LocalReader", "got data control manager");
                state.data_manager = Some(registry.bind(name, version, qh, ()));
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &WlSeat,
        event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _queue: &wayland_client::QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_seat::Event;
        if let Event::Name { .. } = event {
            log::info!(target: "LocalReader", "got seat name");
            state.got_seat_name = true;
        }
    }
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _queue: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: <ZwlrDataControlDeviceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhahdle: &wayland_client::QueueHandle<Self>,
    ) {
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::Event;

        if let Event::Selection { id } = event {
            let Some(offer) = id else {
                return;
            };
            let (reader, writer) = match os_pipe::pipe() {
                Ok(pipe) => pipe,
                Err(err) => {
                    log::error!(target: "LocalReader", "failed to create pipe: {err:?}");
                    return;
                }
            };
            offer.receive(String::from("text/plain;charset=utf-8"), writer.as_fd());
            state.reader = Some(reader);
        }
    }

    event_created_child!(State,
        wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1, [
            wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
                ()
            )
        ]
    );
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlSourceV1,
        _event: <ZwlrDataControlSourceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _queue: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlOfferV1,
        _event: <ZwlrDataControlOfferV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _queue: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
