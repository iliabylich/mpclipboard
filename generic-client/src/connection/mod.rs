use crate::{
    config::Config,
    connection::actions::{
        finish_connecting, finish_tls_handshake, read_message, read_upgrade_response,
        write_message, write_upgrade_request,
    },
};
use mpclipboard_shared::{
    Message, MessageReader, MessageWriter, UpgradeRequestWriter, UpgradeResponseReader, Wants,
    prelude::*,
};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

mod actions;
use actions::reconnect;

mod maybe_tls_stream;
use maybe_tls_stream::MaybeTlsStream;

#[derive(Debug)]
pub enum State {
    Connecting(u64),
    TlsHandshake(u64),
    WritingUpgradeRequest(u64, UpgradeRequestWriter),
    ReadingUpgradeResponse(u64, UpgradeResponseReader),
    Connected(MessageReader, MessageWriter),
}

#[derive(Debug)]
pub enum Connection {
    Disconnected(u64),
    State {
        fd: OwnedFd,
        stream: MaybeTlsStream,
        state: State,
    },
}

impl Connection {
    const FREEZE_TIME_IN_SECS: u64 = 3;

    pub(crate) const fn new() -> Self {
        Self::Disconnected(0)
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::Disconnected { .. } => "Disconnected",
            Self::State { state, .. } => match state {
                State::Connecting { .. } => "Connecting",
                State::TlsHandshake { .. } => "TlsHandshake",
                State::WritingUpgradeRequest { .. } => "WritingUpgradeRequest",
                State::ReadingUpgradeResponse { .. } => "ReadingUpgradeResponse",
                State::Connected { .. } => "Connected",
            },
        }
    }

    pub(crate) fn force_disconnect(&mut self, now: u64) {
        *self = Self::Disconnected(now);
    }

    pub(crate) const fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected { .. })
    }

    fn reconnect(now: u64, config: &Config) -> Self {
        match reconnect(config) {
            Done((fd, stream, writer)) => Self::State {
                fd,
                state: if stream.is_tls() {
                    State::TlsHandshake(now)
                } else {
                    State::WritingUpgradeRequest(now, writer)
                },
                stream,
            },

            Pending((fd, stream)) => Self::State {
                fd,
                stream,
                state: State::Connecting(now),
            },

            Failed(err) => {
                log::error!("failed to connect: {err:?}");
                Self::Disconnected(now)
            }
        }
    }

    pub(crate) fn tick(&mut self, now: u64, config: &Config) {
        match self {
            Self::Disconnected(disconnected_at) => {
                let seconds_passed = now
                    .checked_sub(*disconnected_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    *self = Self::reconnect(now, config);
                }
            }

            Self::State { state, .. } => {
                let last_activity_at = match state {
                    State::Connecting(last_activity_at)
                    | State::TlsHandshake(last_activity_at)
                    | State::WritingUpgradeRequest(last_activity_at, _)
                    | State::ReadingUpgradeResponse(last_activity_at, _) => *last_activity_at,

                    State::Connected { .. } => return,
                };

                let seconds_passed = now
                    .checked_sub(last_activity_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    log::error!("Stuck in {}, disconnecting...", self.name());
                    *self = Self::Disconnected(now);
                }
            }
        }
    }

    pub(crate) fn push(&mut self, message: Message) -> bool {
        let Self::State { state: active, .. } = self else {
            return false;
        };
        let State::Connected(_reader, writer) = active else {
            return false;
        };
        writer.push(&message);
        true
    }

    pub(crate) fn on_readable(&mut self, now: u64, config: &Config) -> Option<Message> {
        match self {
            Self::Disconnected { .. } => {
                unreachable!("can't read() in Disconnected state")
            }

            Self::State { fd, stream, state } => match state {
                State::TlsHandshake(last_activity_at) => {
                    match finish_tls_handshake(stream, fd, config) {
                        Done(writer) => *state = State::WritingUpgradeRequest(now, writer),
                        Pending(()) => *last_activity_at = now,
                        Failed(err) => {
                            log::error!("failed to finish TLS handshake: {err:?}");
                            *self = Self::Disconnected(now);
                        }
                    }
                }

                State::ReadingUpgradeResponse(last_activity_at, reader) => {
                    match read_upgrade_response(fd, stream, reader) {
                        Done(reader) => {
                            *state = State::Connected(reader, MessageWriter::new());
                        }
                        Pending(()) => *last_activity_at = now,
                        Failed(err) => {
                            log::error!("failed to read upgrade response: {err:?}");
                            *self = Self::Disconnected(now);
                        }
                    }
                }

                State::Connected(reader, _writer) => match read_message(reader, stream, fd) {
                    Done(message) => return Some(message),
                    Failed(err) => {
                        log::error!("failed to read message: {err:?}");
                        *self = Self::Disconnected(now);
                    }
                    Pending(()) => {}
                },

                State::Connecting { .. } | State::WritingUpgradeRequest { .. } => {
                    unreachable!("can't read() in {} state", self.name())
                }
            },
        }

        None
    }

    pub(crate) fn on_writable(&mut self, now: u64, config: &Config) {
        match self {
            Self::Disconnected { .. } => {
                unreachable!("can't write() in Disconencted state")
            }

            Self::State {
                fd,
                stream,
                state: active,
            } => match active {
                State::Connecting { .. } => {
                    match (finish_connecting(fd, config), stream.is_tls()) {
                        (Done(_writer), true) => *active = State::TlsHandshake(now),
                        (Done(writer), false) => {
                            *active = State::WritingUpgradeRequest(now, writer);
                        }
                        (Failed(err), _) => {
                            log::error!("failed to finish connecting: {err:?}");
                            *self = Self::Disconnected(now);
                        }
                    }
                }

                State::TlsHandshake(last_activity_at) => {
                    match finish_tls_handshake(stream, fd, config) {
                        Done(writer) => *active = State::WritingUpgradeRequest(now, writer),
                        Pending(()) => *last_activity_at = now,
                        Failed(err) => {
                            log::error!("failed to finish TLS handshake {err:?}");
                            *self = Self::Disconnected(now);
                        }
                    }
                }

                State::WritingUpgradeRequest(last_activity_at, writer) => {
                    match write_upgrade_request(fd, stream, writer) {
                        Done(()) => {
                            *active =
                                State::ReadingUpgradeResponse(now, UpgradeResponseReader::new());
                        }
                        Pending(()) => *last_activity_at = now,
                        Failed(err) => {
                            log::error!("failed to write upgrade request: {err:?}");
                            *self = Self::Disconnected(now);
                        }
                    }
                }

                State::ReadingUpgradeResponse(last_activity_at, _) => match stream.flush(fd) {
                    Ok(()) => *last_activity_at = now,
                    Err(err) => {
                        log::error!("failed to flush TLS data: {err:?}");
                        *self = Self::Disconnected(now);
                    }
                },

                State::Connected(_reader, writer) => match write_message(writer, stream, fd) {
                    Done(()) | Pending(()) => {}
                    Failed(err) => {
                        log::error!("failed to write message: {err:?}");
                        *self = Self::Disconnected(now);
                    }
                },
            },
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'_>, Wants)> {
        match self {
            Self::Disconnected { .. } => None,
            Self::State { state, fd, stream } => {
                let fd = fd.as_fd();
                let wants = match state {
                    State::Connecting { .. } => Wants::Write,
                    State::TlsHandshake { .. } => stream
                        .tls_wants()
                        .unwrap_or_else(|| unreachable!("TlsStream always wants soemthing")),
                    State::WritingUpgradeRequest { .. } => {
                        Wants::Write.merge_opt(stream.tls_wants())
                    }
                    State::ReadingUpgradeResponse { .. } => {
                        Wants::Read.merge_opt(stream.tls_wants())
                    }
                    State::Connected(_reader, writer) => Wants::Read
                        .merge_opt(writer.wants())
                        .merge_opt(stream.tls_wants()),
                };
                Some((fd, wants))
            }
        }
    }
}
