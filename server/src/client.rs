use crate::as_poll_fd::AsPollFd;
use anyhow::anyhow;
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
    ) -> Completion<(Message, Self), anyhow::Error, Self> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                return Failed(anyhow!(err).context(format!("[{self}] polling returned an error")));
            }
        };

        if revents.writable {
            log::trace!("[{self}] is writable");
            match self.write() {
                Done(()) | Pending(()) => {}
                Failed(err) => return Failed(err.context(format!("[{self}] write() failed"))),
            }
        }

        if revents.readable {
            log::trace!("[{self}] is readable");
            match self.read() {
                Done(message) => return Done((message, self)),
                Pending(()) => {}
                Failed(err) => return Failed(err.context(format!("[{self}] read() failed"))),
            }
        }

        Pending(self)
    }

    fn write(&mut self) -> Completion<(), anyhow::Error, ()> {
        let buf = self
            .writer
            .remainder()
            .unwrap_or_else(|| unreachable!("can't write on empty writer"));
        let len = match mpclipboard_shared::io::write(&self.fd, buf) {
            Done(len) => len,
            Pending(()) => return Pending(()),
            Failed(err) => return Failed(err.into()),
        };
        match self.writer.written(len) {
            Ok(()) => Done(()),
            Err(err) => Failed(anyhow!(err)),
        }
    }

    fn read(&mut self) -> Completion<Message, anyhow::Error, ()> {
        let buf = match mpclipboard_shared::io::read(&self.fd) {
            Done(buf) => buf,
            Pending(()) => return Pending(()),
            Failed(err) => return Failed(err.into()),
        };

        self.reader.received(buf).map_err(|err| anyhow!(err))
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
