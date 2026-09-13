use crate::as_poll_fd::AsPollFd;
use mpclipboard_shared::{
    Completion::{self, *},
    ID, Message, MessageReader, MessageWriter, REvents, error, trace,
};
use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use std::{
    num::NonZeroUsize,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
};

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
    ) -> Completion<(Message, Client), Client> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return Failed;
            }
        };

        if revents.writable {
            trace!("{self} is writable");

            let Some(buf) = self.writer.remainder() else {
                unreachable!("empty writer was not polled")
            };
            match rustix::io::write(&self.fd, buf).map(NonZeroUsize::new) {
                Ok(Some(len)) => self.writer.written(len),
                Err(Errno::AGAIN) => {}
                Ok(None) => {
                    error!("write() returned zero for {self}");
                    return Failed;
                }
                Err(errno) => {
                    error!("failed to write() for {self}: {errno:?}");
                    return Failed;
                }
            }
        }

        if revents.readable {
            trace!("{self} is readable");

            let mut buf = [0; Message::BYTESIZE];
            let len = match rustix::io::read(&self.fd, &mut buf).map(NonZeroUsize::new) {
                Ok(Some(len)) => len,
                Err(Errno::AGAIN) => return Pending(self),
                Ok(None) => {
                    error!("{self} reached EOF");
                    return Failed;
                }
                Err(errno) => {
                    error!("failed to read() for {self}: {errno:?}");
                    return Failed;
                }
            };

            match self.reader.received(buf, len) {
                Done(message) => return Done((message, self)),
                Pending(()) => {}
                Failed => return Failed,
            }
        }

        Pending(self)
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
