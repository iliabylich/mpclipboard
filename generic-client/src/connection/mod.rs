use crate::{
    Connectivity,
    config::Config,
    connection::actions::{
        finish_connecting, read_message, read_upgrade_response, write_message,
        write_upgrade_request,
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
enum State {
    Disconnected {
        disconnected_at: u64,
    },
    Active {
        fd: OwnedFd,
        stream: MaybeTlsStream,
        state: ActiveState,
    },
}

#[derive(Debug)]
enum ActiveState {
    Connecting {
        last_activity_at: u64,
    },
    TlsHandshake {
        last_activity_at: u64,
    },
    WritingUpgradeRequest {
        writer: UpgradeRequestWriter,
        last_activity_at: u64,
    },
    ReadingUpgradeResponse {
        reader: UpgradeResponseReader,
        last_activity_at: u64,
    },
    Connected {
        reader: MessageReader,
        writer: MessageWriter,
    },
}

impl State {
    const fn name(&self) -> &'static str {
        match self {
            Self::Disconnected { .. } => "Disconnected",
            Self::Active { state, .. } => match state {
                ActiveState::Connecting { .. } => "Connecting",
                ActiveState::TlsHandshake { .. } => "TlsHandshake",
                ActiveState::WritingUpgradeRequest { .. } => "WritingUpgradeRequest",
                ActiveState::ReadingUpgradeResponse { .. } => "ReadingUpgradeResponse",
                ActiveState::Connected { .. } => "Connected",
            },
        }
    }

    const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Disconnected { .. } => Connectivity::Disconnected,
            Self::Active {
                state: ActiveState::Connected { .. },
                ..
            } => Connectivity::Connected,
            Self::Active { .. } => Connectivity::Connecting,
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    state: State,
    config: Config,
}

impl Connection {
    const FREEZE_TIME_IN_SECS: u64 = 3;

    pub(crate) const fn new(config: Config) -> Self {
        Self {
            state: State::Disconnected { disconnected_at: 0 },
            config,
        }
    }

    fn reconnect(&mut self, now: u64) {
        match reconnect(&self.config) {
            Done((fd, stream)) => {
                if stream.is_tls() {
                    self.state = State::Active {
                        fd,
                        stream,
                        state: ActiveState::TlsHandshake {
                            last_activity_at: now,
                        },
                    }
                } else {
                    self.state = State::Active {
                        fd,
                        stream,
                        state: ActiveState::WritingUpgradeRequest {
                            writer: UpgradeRequestWriter::new(self.config.update_request()),
                            last_activity_at: now,
                        },
                    }
                }
            }

            Pending((fd, stream)) => {
                self.state = State::Active {
                    fd,
                    stream,
                    state: ActiveState::Connecting {
                        last_activity_at: now,
                    },
                };
            }

            Failed(err) => {
                log::error!("failed to connect: {err:?}");
                self.force_disconnect(now);
            }
        }
    }

    pub(crate) fn tick(&mut self, now: u64) {
        match &self.state {
            State::Disconnected { disconnected_at } => {
                let seconds_passed = now
                    .checked_sub(*disconnected_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    self.reconnect(now);
                }
            }

            State::Active { state, .. } => {
                let last_activity_at = match state {
                    ActiveState::Connecting { last_activity_at }
                    | ActiveState::TlsHandshake { last_activity_at }
                    | ActiveState::WritingUpgradeRequest {
                        last_activity_at, ..
                    }
                    | ActiveState::ReadingUpgradeResponse {
                        last_activity_at, ..
                    } => *last_activity_at,

                    ActiveState::Connected { .. } => return,
                };

                let seconds_passed = now
                    .checked_sub(last_activity_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    log::error!("Stuck in {}, disconnecting...", self.state.name());
                    self.force_disconnect(now);
                }
            }
        }
    }

    pub(crate) fn push(&mut self, message: Message) -> bool {
        let State::Active {
            state: ActiveState::Connected { writer, .. },
            ..
        } = &mut self.state
        else {
            return false;
        };
        writer.push(&message);
        true
    }

    pub(crate) fn force_disconnect(&mut self, now: u64) {
        self.state = State::Disconnected {
            disconnected_at: now,
        };
    }

    pub(crate) const fn is_disconnected(&self) -> bool {
        matches!(self.state, State::Disconnected { .. })
    }

    pub(crate) fn on_readable(&mut self, now: u64) -> Option<Message> {
        match &mut self.state {
            State::Disconnected { .. } => {
                unreachable!("can't read() in Disconnected state")
            }

            State::Active { fd, stream, state } => match state {
                ActiveState::TlsHandshake { last_activity_at } => {
                    match stream.finish_tls_handshake(fd) {
                        Done(()) => {
                            *state = ActiveState::WritingUpgradeRequest {
                                writer: UpgradeRequestWriter::new(self.config.update_request()),
                                last_activity_at: now,
                            }
                        }
                        Pending(()) => *last_activity_at = now,
                        Failed(err) => {
                            log::error!("failed to finish TLS handshake: {err:?}");
                            self.force_disconnect(now);
                        }
                    }
                }

                ActiveState::ReadingUpgradeResponse {
                    reader,
                    last_activity_at,
                } => match read_upgrade_response(fd, stream, reader) {
                    Done(reader) => {
                        *state = ActiveState::Connected {
                            reader,
                            writer: MessageWriter::new(),
                        };
                    }
                    Pending(()) => *last_activity_at = now,
                    Failed(err) => {
                        log::error!("failed to read upgrade response: {err:?}");
                        self.force_disconnect(now);
                    }
                },

                ActiveState::Connected { reader, .. } => match read_message(reader, stream, fd) {
                    Done(message) => return Some(message),
                    Failed(err) => {
                        log::error!("failed to read message: {err:?}");
                        self.force_disconnect(now);
                    }
                    Pending(()) => {}
                },

                ActiveState::Connecting { .. } | ActiveState::WritingUpgradeRequest { .. } => {
                    unreachable!("can't read() in {} state", self.state.name())
                }
            },
        }

        None
    }

    pub(crate) fn on_writable(&mut self, now: u64) {
        match &mut self.state {
            State::Disconnected { .. } => {
                unreachable!("can't write() in Disconencted state")
            }

            State::Active { fd, stream, state } => match state {
                ActiveState::Connecting { .. } => match (finish_connecting(fd), stream.is_tls()) {
                    (Done(()), true) => {
                        *state = ActiveState::TlsHandshake {
                            last_activity_at: now,
                        }
                    }
                    (Done(()), false) => {
                        *state = ActiveState::WritingUpgradeRequest {
                            writer: UpgradeRequestWriter::new(self.config.update_request()),
                            last_activity_at: now,
                        }
                    }
                    (Failed(err), _) => {
                        log::error!("failed to finish connecting: {err:?}");
                        self.force_disconnect(now);
                    }
                },

                ActiveState::TlsHandshake { last_activity_at } => {
                    match stream.finish_tls_handshake(fd) {
                        Done(()) => {
                            *state = ActiveState::WritingUpgradeRequest {
                                writer: UpgradeRequestWriter::new(self.config.update_request()),
                                last_activity_at: now,
                            }
                        }
                        Pending(()) => *last_activity_at = now,
                        Failed(err) => {
                            log::error!("failed to finish TLS handshake {err:?}");
                            self.force_disconnect(now);
                        }
                    }
                }

                ActiveState::WritingUpgradeRequest {
                    writer,
                    last_activity_at,
                } => match write_upgrade_request(fd, stream, writer) {
                    Done(()) => {
                        *state = ActiveState::ReadingUpgradeResponse {
                            reader: UpgradeResponseReader::new(),
                            last_activity_at: now,
                        };
                    }
                    Pending(()) => *last_activity_at = now,
                    Failed(err) => {
                        log::error!("failed to write upgrade request: {err:?}");
                        self.force_disconnect(now);
                    }
                },

                ActiveState::ReadingUpgradeResponse {
                    last_activity_at, ..
                } => match stream.flush(fd) {
                    Ok(()) => *last_activity_at = now,
                    Err(err) => {
                        log::error!("failed to flush TLS data: {err:?}");
                        self.force_disconnect(now);
                    }
                },

                ActiveState::Connected { writer, .. } => match write_message(writer, stream, fd) {
                    Done(()) | Pending(()) => {}
                    Failed(err) => {
                        log::error!("failed to write message: {err:?}");
                        self.force_disconnect(now);
                    }
                },
            },
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'_>, Wants)> {
        match &self.state {
            State::Disconnected { .. } => None,
            State::Active { state, fd, stream } => {
                let fd = fd.as_fd();
                let wants = match state {
                    ActiveState::Connecting { .. } => Wants::Write,
                    ActiveState::TlsHandshake { .. } => stream
                        .tls_wants()
                        .unwrap_or_else(|| unreachable!("TlsStream always wants soemthing")),
                    ActiveState::WritingUpgradeRequest { .. } => {
                        Wants::Write.merge_opt(stream.tls_wants())
                    }
                    ActiveState::ReadingUpgradeResponse { .. } => {
                        Wants::Read.merge_opt(stream.tls_wants())
                    }
                    ActiveState::Connected { writer, .. } => Wants::Read
                        .merge_opt(writer.wants())
                        .merge_opt(stream.tls_wants()),
                };
                Some((fd, wants))
            }
        }
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        self.state.connectivity()
    }
}
