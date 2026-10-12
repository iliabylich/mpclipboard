use crate::as_poll_fd::AsPollFd;
use anyhow::{Context, Result};
use mpclipboard_shared::{ID, Message, MessageReader, MessageWriter, REvents, prelude::*};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct Client {
    fd: OwnedFd,
    id: ID,
    reader: MessageReader,
    writer: MessageWriter,
}

impl Client {
    pub(crate) const fn new(fd: OwnedFd, id: ID) -> Self {
        Self {
            fd,
            id,
            reader: MessageReader::empty(),
            writer: MessageWriter::new(),
        }
    }

    pub(crate) fn push(&mut self, message: &Message) {
        self.writer.push(message);
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: PollFlags,
    ) -> Result<Completion<(Message, Self), Self>> {
        let revents =
            REvents::new(revents).with_context(|| format!("[{self}] polling returned an error"))?;

        if revents.writable {
            log::trace!("[{self}] is writable");
            let _ = self
                .write()
                .with_context(|| format!("[{self}] write() failed"))?;
        }

        if revents.readable {
            log::trace!("[{self}] is readable");
            if let Done(message) = self
                .read()
                .with_context(|| format!("[{self}] read() failed"))?
            {
                return Ok(Done((message, self)));
            }
        }

        Ok(Pending(self))
    }

    fn write(&mut self) -> Result<Completion<(), ()>> {
        let buf = self
            .writer
            .remainder()
            .unwrap_or_else(|| unreachable!("can't write on empty writer"));
        let Done(len) = mpclipboard_shared::io::write(&self.fd, buf)? else {
            return Ok(Pending(()));
        };
        self.writer.written(len)?;
        Ok(Done(()))
    }

    fn read(&mut self) -> Result<Completion<Message, ()>> {
        let Done(buf) = mpclipboard_shared::io::read(&self.fd)? else {
            return Ok(Pending(()));
        };

        let received = self.reader.received(buf)?;
        Ok(received)
    }

    pub(crate) const fn id(&self) -> ID {
        self.id
    }
}

impl core::fmt::Display for Client {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Client(fd={}, id={})", self.fd.as_raw_fd(), self.id)
    }
}

impl AsFd for Client {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Client {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for Client {
    fn as_poll_fd(&self) -> PollFd<'_> {
        let mut events = PollFlags::IN;
        if !self.writer.is_empty() {
            events |= PollFlags::OUT;
        }
        PollFd::new(&self.fd, events)
    }
}
