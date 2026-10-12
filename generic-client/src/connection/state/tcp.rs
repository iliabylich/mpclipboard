use crate::{
    Connectivity,
    config::Config,
    connection::{
        actions::{
            connect, finish_connecting, finish_tls_handshake, read_message, read_upgrade_response,
            write_message, write_upgrade_request,
        },
        error::ConnectionError,
        maybe_tls_stream::MaybeTlsStream,
        state::{FREEZE_TIME_IN_SECS, State},
    },
};
use core::net::SocketAddr;
use mpclipboard_shared::{
    Message, MessageReader, MessageWriter, OptionWantsExt, UpgradeRequestWriter,
    UpgradeResponseReader, Wants, prelude::*,
};
use rustix::fd::OwnedFd;

#[derive(Debug)]
pub enum TcpState {
    Connecting(u64),
    TlsHandshake(u64),
    WritingUpgradeRequest(u64, UpgradeRequestWriter),
    ReadingUpgradeResponse(u64, UpgradeResponseReader),
    Connected(MessageReader, MessageWriter),
}

impl TcpState {
    pub(crate) fn connect(
        now: u64,
        addr: SocketAddr,
        config: &Config,
    ) -> Result<State, ConnectionError> {
        let (fd, stream, state) = match connect(addr, config)? {
            Done((fd, stream)) => {
                let state = if stream.is_tls() {
                    Self::TlsHandshake(now)
                } else {
                    let writer = UpgradeRequestWriter::new(config.update_request());
                    Self::WritingUpgradeRequest(now, writer)
                };
                (fd, stream, state)
            }
            Pending((fd, stream)) => (fd, stream, Self::Connecting(now)),
        };

        Ok(State::Tcp { fd, stream, state })
    }

    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Connecting { .. } => "Connecting",
            Self::TlsHandshake { .. } => "TlsHandshake",
            Self::WritingUpgradeRequest { .. } => "WritingUpgradeRequest",
            Self::ReadingUpgradeResponse { .. } => "ReadingUpgradeResponse",
            Self::Connected { .. } => "Connected",
        }
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Connecting { .. }
            | Self::TlsHandshake { .. }
            | Self::WritingUpgradeRequest { .. }
            | Self::ReadingUpgradeResponse { .. } => Connectivity::Connecting,
            Self::Connected { .. } => Connectivity::Connected,
        }
    }

    pub(crate) fn is_stuck(&self, now: u64) -> bool {
        let last_activity_at = match self {
            Self::Connecting(last_activity_at)
            | Self::TlsHandshake(last_activity_at)
            | Self::WritingUpgradeRequest(last_activity_at, _)
            | Self::ReadingUpgradeResponse(last_activity_at, _) => *last_activity_at,

            Self::Connected { .. } => return false,
        };

        let seconds_passed = now
            .checked_sub(last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));

        seconds_passed > FREEZE_TIME_IN_SECS
    }

    pub(crate) fn on_readable(
        &mut self,
        fd: &OwnedFd,
        stream: &mut MaybeTlsStream,
        now: u64,
        config: &Config,
    ) -> Result<Completion<Message, ()>, ConnectionError> {
        match self {
            Self::TlsHandshake(last_activity_at) => {
                let Done(()) = finish_tls_handshake(stream, fd)? else {
                    *last_activity_at = now;
                    return Ok(Pending(()));
                };

                let writer = UpgradeRequestWriter::new(config.update_request());
                *self = Self::WritingUpgradeRequest(now, writer);
                Ok(Pending(()))
            }

            Self::ReadingUpgradeResponse(last_activity_at, reader) => {
                let Done(mut reader) = read_upgrade_response(fd, stream, reader)? else {
                    *last_activity_at = now;
                    return Ok(Pending(()));
                };

                let message = read_message(&mut reader, stream, fd)?;
                *self = Self::Connected(reader, MessageWriter::new());
                Ok(message)
            }

            Self::Connected(reader, _writer) => read_message(reader, stream, fd),

            Self::Connecting { .. } => {
                unreachable!("can't read() in Connecting state")
            }

            Self::WritingUpgradeRequest { .. } => {
                unreachable!("can't read() in WritingUpgradeRequest state")
            }
        }
    }

    pub(crate) fn on_writable(
        &mut self,
        fd: &OwnedFd,
        stream: &mut MaybeTlsStream,
        now: u64,
        config: &Config,
    ) -> Result<(), ConnectionError> {
        match self {
            Self::Connecting(_) => {
                finish_connecting(fd).map_err(ConnectionError::FailedToFinishConnecting)?;

                *self = if stream.is_tls() {
                    Self::TlsHandshake(now)
                } else {
                    let writer = UpgradeRequestWriter::new(config.update_request());
                    Self::WritingUpgradeRequest(now, writer)
                };
            }

            Self::TlsHandshake(last_activity_at) => {
                let Done(()) = finish_tls_handshake(stream, fd)? else {
                    *last_activity_at = now;
                    return Ok(());
                };

                let writer = UpgradeRequestWriter::new(config.update_request());
                *self = Self::WritingUpgradeRequest(now, writer);
            }

            Self::WritingUpgradeRequest(last_activity_at, writer) => {
                let Done(()) = write_upgrade_request(fd, stream, writer)? else {
                    *last_activity_at = now;
                    return Ok(());
                };

                *self = Self::ReadingUpgradeResponse(now, UpgradeResponseReader::new());
            }

            Self::ReadingUpgradeResponse(last_activity_at, _reader) => {
                stream
                    .flush(fd)
                    .map_err(ConnectionError::FailedToFlushTls)?;
                *last_activity_at = now;
            }

            Self::Connected(_reader, writer) => {
                let (Done(()) | Pending(())) = write_message(writer, stream, fd)?;
            }
        }

        Ok(())
    }

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> Wants {
        match self {
            Self::Connecting { .. } => Wants::Write,
            Self::TlsHandshake { .. } => stream
                .tls_wants_read()
                .merge_opt(stream.tls_wants_write())
                .unwrap_or_else(|| unreachable!("TlsStream always wants soemthing")),
            Self::WritingUpgradeRequest { .. } => Wants::Write.merge_opt(stream.tls_wants_write()),
            Self::ReadingUpgradeResponse { .. } => Wants::Read.merge_opt(stream.tls_wants_write()),
            Self::Connected(_reader, writer) => Wants::Read
                .merge_opt(writer.wants())
                .merge_opt(stream.tls_wants_write()),
        }
    }
}
