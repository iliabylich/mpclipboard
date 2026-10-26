use crate::dbus::queue::DBusQueue;
use anyhow::{Context, Result, bail, ensure};
use mini_sansio_dbus::{
    DBusConnection, DBusConnector, DBusConnectorWants, DBusWantsRead, DBusWantsWrite,
    IncomingMessage,
};
use rustix::{
    event::{PollFd, PollFlags},
    fs::OFlags,
    net::{AddressFamily, SocketAddrUnix, SocketType},
};
use std::{
    io::ErrorKind,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};

struct Connecting {
    connector: DBusConnector,
}
impl Connecting {
    fn wants_read(&self, readbuf: &mut [u8]) -> Result<bool> {
        Ok(matches!(
            self.connector.wants(readbuf)?,
            DBusConnectorWants::Read { .. }
        ))
    }

    fn wants_write(&self, readbuf: &mut [u8]) -> Result<bool> {
        Ok(matches!(
            self.connector.wants(readbuf)?,
            DBusConnectorWants::Write { .. }
        ))
    }

    fn read(&mut self, fd: BorrowedFd<'_>, readbuf: &mut [u8]) -> Result<()> {
        match self.connector.wants(readbuf)? {
            DBusConnectorWants::Read { buf, .. } => {
                log::trace!(target: "DBusConnector", "reading {}", buf.len());
                let len = match rustix::io::read(fd, buf) {
                    Ok(v) => v,
                    Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
                    Err(err) => return Err(err.into()),
                };
                self.connector.satisfy_read(len, readbuf)?;
            }
            DBusConnectorWants::Write { .. } => {
                bail!("DBus(in Connecting)::read() called when it doesn't want to read");
            }
        }
        Ok(())
    }

    fn write(&mut self, fd: BorrowedFd<'_>, readbuf: &mut [u8]) -> Result<bool> {
        match self.connector.wants(readbuf)? {
            DBusConnectorWants::Write { buf, .. } => {
                log::trace!(target: "DBusConnector", "writing {}", buf.len());
                let len = match rustix::io::write(fd, buf) {
                    Ok(v) => v,
                    Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(false),
                    Err(err) => return Err(err.into()),
                };
                if self.connector.satisfy_write(len)?.is_some() {
                    return Ok(true);
                }
            }
            DBusConnectorWants::Read { .. } => {
                bail!("DBus(in Connecting)::write() called when it doesn't want to read");
            }
        }

        Ok(false)
    }
}

struct Connected {
    conn: DBusConnection,
}
impl Connected {
    const fn wants_read() -> bool {
        true
    }

    fn wants_write(&self, readbuf: &mut [u8], queue: &DBusQueue) -> Result<bool> {
        let (_read, write) = self.conn.wants(queue, readbuf)?;
        Ok(write.is_some())
    }

    fn read<'r>(
        &mut self,
        fd: BorrowedFd<'_>,
        readbuf: &'r mut [u8],
        queue: &DBusQueue,
    ) -> Result<Option<IncomingMessage<'r>>> {
        let (DBusWantsRead { buf, .. }, _write) = self.conn.wants(queue, readbuf)?;
        log::trace!(target: "DBusConnection", "reading {}", buf.len());
        let len = match rustix::io::read(fd, buf) {
            Ok(v) => v,
            Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let message = self.conn.satisfy_read(len, readbuf)?;
        Ok(message)
    }

    fn write(
        &mut self,
        fd: BorrowedFd<'_>,
        readbuf: &mut [u8],
        queue: &mut DBusQueue,
    ) -> Result<()> {
        let (_read, write) = self.conn.wants(queue, readbuf)?;

        let Some(DBusWantsWrite { buf, .. }) = write else {
            bail!("DBus(in Connected)::write() called when it doesn't want to read");
        };

        log::trace!(target: "DBusConnection", "writing {}", buf.len());
        let len = match rustix::io::write(fd, buf) {
            Ok(v) => v,
            Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        self.conn.satisfy_write(len, queue)?;
        Ok(())
    }
}

pub(crate) struct DBus {
    fd: OwnedFd,
    state: State,
}

enum State {
    Connecting(Connecting),
    Connected(Connected),
}

impl DBus {
    fn address() -> Result<SocketAddrUnix> {
        let address =
            std::env::var("DBUS_SESSION_BUS_ADDRESS").context("no $DBUS_SESSION_BUS_ADDRESS")?;
        let (_, path) = address
            .split_once('=')
            .context("malformed $DBUS_SESSION_BUS_ADDRESS")?;
        let sockaddr = SocketAddrUnix::new(path).context("failed to create sockaddr")?;
        Ok(sockaddr)
    }

    pub(crate) fn new() -> Result<Self> {
        let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)
            .context("failed to socket()")?;

        let mut flags = rustix::fs::fcntl_getfl(&fd)?;
        flags.insert(OFlags::NONBLOCK);
        rustix::fs::fcntl_setfl(&fd, flags)?;

        let addr = Self::address()?;
        rustix::net::connect(&fd, &addr).context("failed to connect()")?;
        Ok(Self {
            fd,
            state: State::Connecting(Connecting {
                connector: DBusConnector::new(),
            }),
        })
    }

    pub(crate) fn as_pollfd(&self, readbuf: &mut [u8], queue: &DBusQueue) -> Result<PollFd<'_>> {
        let mut flags = PollFlags::empty();
        if self.wants_read(readbuf)? {
            flags |= PollFlags::IN;
        }
        if self.wants_write(readbuf, queue)? {
            flags |= PollFlags::OUT;
        }
        ensure!(!flags.is_empty(), "DBus wants nothing");
        Ok(PollFd::new(&self.fd, flags))
    }

    pub(crate) fn wants_read(&self, readbuf: &mut [u8]) -> Result<bool> {
        match &self.state {
            State::Connecting(connecting) => connecting.wants_read(readbuf),
            State::Connected(_) => Ok(Connected::wants_read()),
        }
    }

    pub(crate) fn read<'r>(
        &mut self,
        readbuf: &'r mut [u8],
        queue: &DBusQueue,
    ) -> Result<Option<IncomingMessage<'r>>> {
        match &mut self.state {
            State::Connecting(connecting) => {
                connecting.read(self.fd.as_fd(), readbuf)?;
                Ok(None)
            }
            State::Connected(connected) => connected.read(self.fd.as_fd(), readbuf, queue),
        }
    }

    pub(crate) fn wants_write(&self, readbuf: &mut [u8], queue: &DBusQueue) -> Result<bool> {
        match &self.state {
            State::Connecting(connecting) => connecting.wants_write(readbuf),
            State::Connected(connected) => connected.wants_write(readbuf, queue),
        }
    }

    pub(crate) fn write(&mut self, readbuf: &mut [u8], queue: &mut DBusQueue) -> Result<()> {
        match &mut self.state {
            State::Connecting(connecting) => {
                let done = connecting.write(self.fd.as_fd(), readbuf)?;
                if done {
                    self.state = State::Connected(Connected {
                        conn: DBusConnection::new(0),
                    });
                }
            }
            State::Connected(connected) => {
                connected.write(self.fd.as_fd(), readbuf, queue)?;
            }
        }
        Ok(())
    }
}

impl AsFd for DBus {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
