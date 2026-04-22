use crate::event_loop::EventLoop;
use anyhow::Result;
use std::{
    io::ErrorKind,
    net::TcpStream,
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    rc::Rc,
};
use tungstenite::{Message, stream::MaybeTlsStream};

/// cbindgen:ignore
type WebSocket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;

pub(crate) struct EventLoopAwareWebSocket {
    inner: WebSocket,
    fd: i32,
    event_loop: Rc<EventLoop>,
}

impl EventLoopAwareWebSocket {
    pub(crate) fn new(inner: WebSocket, fd: i32, event_loop: Rc<EventLoop>) -> Self {
        Self {
            inner,
            fd,
            event_loop,
        }
    }

    pub(crate) fn can_read(&self) -> bool {
        self.inner.can_read()
    }
    pub(crate) fn can_write(&self) -> bool {
        self.inner.can_write()
    }

    pub(crate) fn write(&mut self, message: Message) -> Result<WriteResult> {
        if !self.inner.can_write() {
            return Ok(WriteResult::DeadEnd);
        }

        match self.inner.write(message) {
            Ok(()) => {}
            Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(WriteResult::WouldBlock);
            }
            Err(tungstenite::Error::WriteBufferFull(message)) => {
                return Ok(WriteResult::QueueIsFull(*message));
            }
            Err(err) => return Err(anyhow::anyhow!(err)),
        }

        match self.inner.flush() {
            Ok(()) => {}
            Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(WriteResult::WouldBlock);
            }
            Err(tungstenite::Error::WriteBufferFull(message)) => {
                return Ok(WriteResult::QueueIsFull(*message));
            }
            Err(err) => return Err(anyhow::anyhow!(err)),
        }

        Ok(WriteResult::Done)
    }

    pub(crate) fn read(&mut self) -> Result<ReadResult> {
        match self.inner.read() {
            Ok(message) => Ok(ReadResult::Message(message)),
            Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                Ok(ReadResult::WouldBlock)
            }
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
}

pub(crate) enum WriteResult {
    DeadEnd,
    QueueIsFull(Message),
    Done,
    WouldBlock,
}

pub(crate) enum ReadResult {
    Message(Message),
    WouldBlock,
}

impl AsRawFd for EventLoopAwareWebSocket {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd
    }
}

impl AsFd for EventLoopAwareWebSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

impl Drop for EventLoopAwareWebSocket {
    fn drop(&mut self) {
        log::trace!("EventLoopAwareWebSocket::drop()");
        self.event_loop.remove(self.fd);
    }
}
