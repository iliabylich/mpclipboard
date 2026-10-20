use crate::chunked_decoder::ChunkedDecoder;
use anyhow::{Context, Result, ensure};
use mpclipboard_shared::{
    EventLoopFdResult, ID, Message, MessageReader, MessageWriter, RegisteredFd, Wants, prelude::*,
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};

pub struct Client {
    fd: RegisteredFd,
    id: ID,
    decoder: ChunkedDecoder,
    reader: MessageReader,
    writer: MessageWriter,
}

impl Client {
    const READ_SIZE: usize = 8192;

    pub(crate) fn new(fd: RegisteredFd, id: ID, leftover: &[u8]) -> Result<Self> {
        let mut decoder = ChunkedDecoder::default();
        let partial = decoder
            .decode(leftover)
            .with_context(|| format!("[{id}] failed to decode leftover"))?;

        let this = Self {
            fd,
            id,
            decoder,
            reader: MessageReader::new(&partial),
            writer: MessageWriter::default(),
        };
        this.sync_wants()?;
        Ok(this)
    }

    pub(crate) fn push(&mut self, message: &Message) -> Result<()> {
        self.writer.push(message);
        self.sync_wants()
    }

    pub(crate) fn on_poll_event(
        mut self,
        revents: EventLoopFdResult,
    ) -> Result<Completion<(Message, Self), Self>> {
        ensure!(!revents.has_error, "[{self}] polling returned an error");

        if revents.writable && !self.writer.is_empty() {
            log::trace!("[{self}] is writable");
            let _ = self
                .write()
                .with_context(|| format!("[{self}] write() failed"))?;
            self.sync_wants()?;
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
        let Done(buf) = mpclipboard_shared::io::read::<{ Self::READ_SIZE }>(&self.fd)? else {
            return Ok(Pending(()));
        };

        let buf = self.decoder.decode(&buf)?;
        let received = self.reader.received(&buf)?;
        Ok(received)
    }

    pub(crate) const fn id(&self) -> &ID {
        &self.id
    }

    fn sync_wants(&self) -> Result<()> {
        let wants = if self.writer.is_empty() {
            Wants::Read
        } else {
            Wants::ReadWrite
        };
        self.fd
            .modify(wants)
            .with_context(|| format!("[{self}] failed to update interest"))
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
