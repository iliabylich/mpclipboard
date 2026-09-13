use crate::{
    Connectivity,
    config::Config,
    connection::{
        actions::{
            FinishConnectingResult, ReadMessageResult, ReadUpgradeResponseResult,
            WriteMessageResult, WriteUpgradeRequestResult, finish_connecting, read_message,
            read_upgrade_response, write_message, write_upgrade_request,
        },
        maybe_tls_stream::TlsHandshakeResult,
    },
};
use mpclipboard_shared::{
    Message, MessageReader, MessageWriter, UpgradeRequestWriter, UpgradeResponseReader, Wants,
    error,
};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

mod actions;
use actions::{ReconnectResult, reconnect};

mod maybe_tls_stream;
use maybe_tls_stream::MaybeTlsStream;

#[derive(Debug)]
pub enum ConnectionState {
    Disconnected {
        disconnected_at: u64,
    },
    Active {
        fd: OwnedFd,
        stream: MaybeTlsStream,
        state: ActiveConnectionState,
    },
}

#[derive(Debug)]
pub enum ActiveConnectionState {
    Connecting {
        started_at: u64,
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

impl ConnectionState {
    const fn name(&self) -> &'static str {
        match self {
            Self::Disconnected { .. } => "Disconnected",
            Self::Active { state, .. } => match state {
                ActiveConnectionState::Connecting { .. } => "Connecting",
                ActiveConnectionState::TlsHandshake { .. } => "TlsHandshake",
                ActiveConnectionState::WritingUpgradeRequest { .. } => "WritingUpgradeRequest",
                ActiveConnectionState::ReadingUpgradeResponse { .. } => "ReadingUpgradeResponse",
                ActiveConnectionState::Connected { .. } => "Connected",
            },
        }
    }

    const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Disconnected { .. } => Connectivity::Disconnected,
            Self::Active {
                state: ActiveConnectionState::Connected { .. },
                ..
            } => Connectivity::Connected,
            _ => Connectivity::Connecting,
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    state: ConnectionState,
    config: Config,
}

impl Connection {
    const FREEZE_TIME_IN_SECS: u64 = 3;

    pub(crate) const fn new(config: Config) -> Self {
        Self {
            state: ConnectionState::Disconnected { disconnected_at: 0 },
            config,
        }
    }

    fn reconnect(&mut self, now: u64) {
        match reconnect(&self.config) {
            ReconnectResult::Failed => {
                self.state = ConnectionState::Disconnected {
                    disconnected_at: now,
                };
            }
            ReconnectResult::Connecting { fd, stream } => {
                self.state = ConnectionState::Active {
                    fd,
                    stream,
                    state: ActiveConnectionState::Connecting { started_at: now },
                };
            }
            ReconnectResult::ConnectedNeedsTlsHandshake { fd, stream } => {
                self.state = ConnectionState::Active {
                    fd,
                    stream,
                    state: ActiveConnectionState::TlsHandshake {
                        last_activity_at: now,
                    },
                }
            }
            ReconnectResult::ConnectedReadyStartHandshake { fd, stream } => {
                self.state = ConnectionState::Active {
                    fd,
                    stream,
                    state: ActiveConnectionState::WritingUpgradeRequest {
                        writer: UpgradeRequestWriter::new(self.config.update_request()),
                        last_activity_at: now,
                    },
                }
            }
        }
    }

    pub(crate) fn tick(&mut self, now: u64) {
        match &self.state {
            ConnectionState::Disconnected { disconnected_at } => {
                let seconds_passed = now
                    .checked_sub(*disconnected_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    self.reconnect(now);
                }
            }

            ConnectionState::Active { state, .. } => {
                let last_activity_at = match state {
                    ActiveConnectionState::Connecting { started_at } => *started_at,
                    ActiveConnectionState::TlsHandshake { last_activity_at } => *last_activity_at,
                    ActiveConnectionState::WritingUpgradeRequest {
                        last_activity_at, ..
                    } => *last_activity_at,
                    ActiveConnectionState::ReadingUpgradeResponse {
                        last_activity_at, ..
                    } => *last_activity_at,
                    ActiveConnectionState::Connected { .. } => return,
                };

                let seconds_passed = now
                    .checked_sub(last_activity_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    error!("Stuck in {}, disconnecting...", self.state.name());
                    self.force_disconnect(now);
                }
            }
        }
    }

    pub(crate) fn push(&mut self, message: Message) -> bool {
        let ConnectionState::Active {
            state: ActiveConnectionState::Connected { writer, .. },
            ..
        } = &mut self.state
        else {
            return false;
        };
        writer.push(&message);
        true
    }

    pub(crate) fn force_disconnect(&mut self, now: u64) {
        self.state = ConnectionState::Disconnected {
            disconnected_at: now,
        };
    }

    pub(crate) const fn is_disconnected(&self) -> bool {
        matches!(self.state, ConnectionState::Disconnected { .. })
    }

    pub(crate) fn on_readable(&mut self, now: u64) -> Option<Message> {
        match &mut self.state {
            ConnectionState::Disconnected { .. } => {
                unreachable!("can't read() in Disconnected state")
            }

            ConnectionState::Active { fd, stream, state } => match state {
                ActiveConnectionState::TlsHandshake { last_activity_at } => {
                    match stream.finish_tls_handshake(fd) {
                        TlsHandshakeResult::Done => {
                            *state = ActiveConnectionState::WritingUpgradeRequest {
                                writer: UpgradeRequestWriter::new(self.config.update_request()),
                                last_activity_at: now,
                            }
                        }
                        TlsHandshakeResult::Pending => *last_activity_at = now,
                        TlsHandshakeResult::Died => self.force_disconnect(now),
                    }
                }

                ActiveConnectionState::ReadingUpgradeResponse {
                    reader,
                    last_activity_at,
                } => match read_upgrade_response(fd, stream, reader) {
                    ReadUpgradeResponseResult::Done { reader } => {
                        *state = ActiveConnectionState::Connected {
                            reader,
                            writer: MessageWriter::new(),
                        };
                    }
                    ReadUpgradeResponseResult::Pending => *last_activity_at = now,
                    ReadUpgradeResponseResult::Error => self.force_disconnect(now),
                },

                ActiveConnectionState::Connected { reader, .. } => {
                    match read_message(reader, stream, fd) {
                        ReadMessageResult::Done { message } => return Some(message),
                        ReadMessageResult::Pending => {}
                        ReadMessageResult::Error => self.force_disconnect(now),
                    }
                }

                ActiveConnectionState::Connecting { .. }
                | ActiveConnectionState::WritingUpgradeRequest { .. } => {
                    unreachable!("can't read() in {} state", self.state.name())
                }
            },
        }

        None
    }

    pub(crate) fn on_writable(&mut self, now: u64) {
        match &mut self.state {
            ConnectionState::Disconnected { .. } => {
                unreachable!("can't write() in Disconencted state")
            }

            ConnectionState::Active { fd, stream, state } => match state {
                ActiveConnectionState::Connecting { .. } => {
                    match (finish_connecting(fd), stream.is_tls()) {
                        (FinishConnectingResult::Connected, true) => {
                            *state = ActiveConnectionState::TlsHandshake {
                                last_activity_at: now,
                            }
                        }
                        (FinishConnectingResult::Connected, false) => {
                            *state = ActiveConnectionState::WritingUpgradeRequest {
                                writer: UpgradeRequestWriter::new(self.config.update_request()),
                                last_activity_at: now,
                            }
                        }
                        (FinishConnectingResult::FailedToConnect, _) => self.force_disconnect(now),
                    }
                }

                ActiveConnectionState::TlsHandshake { last_activity_at } => {
                    match stream.finish_tls_handshake(fd) {
                        TlsHandshakeResult::Done => {
                            *state = ActiveConnectionState::WritingUpgradeRequest {
                                writer: UpgradeRequestWriter::new(self.config.update_request()),
                                last_activity_at: now,
                            }
                        }
                        TlsHandshakeResult::Pending => *last_activity_at = now,
                        TlsHandshakeResult::Died => self.force_disconnect(now),
                    }
                }

                ActiveConnectionState::WritingUpgradeRequest {
                    writer,
                    last_activity_at,
                } => match write_upgrade_request(fd, stream, writer) {
                    WriteUpgradeRequestResult::Done => {
                        *state = ActiveConnectionState::ReadingUpgradeResponse {
                            reader: UpgradeResponseReader::new(),
                            last_activity_at: now,
                        };
                    }
                    WriteUpgradeRequestResult::Pending => *last_activity_at = now,
                    WriteUpgradeRequestResult::Error => self.force_disconnect(now),
                },

                ActiveConnectionState::ReadingUpgradeResponse {
                    last_activity_at, ..
                } => match stream.flush(fd) {
                    Ok(()) => *last_activity_at = now,
                    Err(err) => {
                        error!("failed to flush TLS data: {err:?}");
                        self.force_disconnect(now);
                    }
                },

                ActiveConnectionState::Connected { writer, .. } => {
                    match write_message(writer, stream, fd) {
                        WriteMessageResult::Ok => {}
                        WriteMessageResult::Error => self.force_disconnect(now),
                    }
                }
            },
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'_>, Wants)> {
        match &self.state {
            ConnectionState::Disconnected { .. } => None,
            ConnectionState::Active { state, fd, stream } => {
                let fd = fd.as_fd();
                let wants = match state {
                    ActiveConnectionState::Connecting { .. } => Wants::Write,
                    ActiveConnectionState::TlsHandshake { .. } => stream
                        .tls_wants()
                        .unwrap_or_else(|| unreachable!("TlsStream always wants soemthing")),
                    ActiveConnectionState::WritingUpgradeRequest { .. } => {
                        Wants::Write.merge_opt(stream.tls_wants())
                    }
                    ActiveConnectionState::ReadingUpgradeResponse { .. } => {
                        Wants::Read.merge_opt(stream.tls_wants())
                    }
                    ActiveConnectionState::Connected { writer, .. } => Wants::Read
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
