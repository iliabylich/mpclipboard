use crate::as_poll_fd::AsPollFd;
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
            Err(err) => return Failed(err.context(format!("polling {self} returned an error"))),
        };

        if revents.writable {
            log::trace!("{self} is writable");
            match self.write() {
                Done(()) | Pending(()) => {}
                Failed(err) => return Failed(err),
            }
        }

        if revents.readable {
            log::trace!("{self} is readable");
            match self.read() {
                Done(message) => return Done((message, self)),
                Pending(()) => {}
                Failed(err) => return Failed(err),
            }
        }

        Pending(self)
    }

    fn write(&mut self) -> Completion<(), anyhow::Error, ()> {
        let Some(buf) = self.writer.remainder() else {
            unreachable!("can't write on empty writer")
        };
        match mpclipboard_shared::io::write(&self.fd, buf) {
            Done(len) => {
                self.writer.written(len);
                Done(())
            }
            Pending(()) => Pending(()),
            Failed(err) => Failed(err.context(format!("write() failed for {self}"))),
        }
    }

    fn read(&mut self) -> Completion<Message, anyhow::Error, ()> {
        let mut buf = [0; Message::BYTESIZE];
        let len = match mpclipboard_shared::io::read(&self.fd, &mut buf) {
            Done(len) => len,
            Pending(()) => return Pending(()),
            Failed(err) => return Failed(err.context(format!("read() failed for {self}"))),
        };

        self.reader.received(buf, len)
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
