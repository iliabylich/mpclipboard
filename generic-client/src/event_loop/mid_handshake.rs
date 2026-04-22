use http_serde::http;
use std::{
    net::TcpStream,
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    rc::Rc,
};
use tungstenite::{ClientHandshake, stream::MaybeTlsStream};

use crate::event_loop::EventLoop;

/// cbindgen:ignore
type WebSocket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;
/// cbindgen:ignore
type MidHandshake =
    tungstenite::handshake::MidHandshake<ClientHandshake<MaybeTlsStream<TcpStream>>>;
/// cbindgen:ignore
type HandshakeResponse = http::Response<Option<Vec<u8>>>;
/// cbindgen:ignore
type HandshakeError = tungstenite::HandshakeError<ClientHandshake<MaybeTlsStream<TcpStream>>>;

pub(crate) struct EventLoopAwareMidHandshake {
    inner: Option<MidHandshake>,
    fd: i32,
    event_loop: Rc<EventLoop>,
}

impl EventLoopAwareMidHandshake {
    pub(crate) fn new(inner: MidHandshake, fd: i32, event_loop: Rc<EventLoop>) -> Self {
        Self {
            inner: Some(inner),
            fd,
            event_loop,
        }
    }

    pub(crate) fn handshake(mut self) -> Result<(WebSocket, HandshakeResponse), HandshakeError> {
        self.inner.take().unwrap().handshake()
    }

    pub(crate) fn event_loop(&self) -> Rc<EventLoop> {
        Rc::clone(&self.event_loop)
    }
}

impl AsRawFd for EventLoopAwareMidHandshake {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd
    }
}

impl AsFd for EventLoopAwareMidHandshake {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

impl Drop for EventLoopAwareMidHandshake {
    fn drop(&mut self) {
        if self.inner.is_none() {
            return;
        }
        log::trace!("EventLoopAwareMidHandshake::drop()");
        self.event_loop.remove(self.fd);
    }
}
