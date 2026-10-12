use crate::{
    Connectivity,
    config::Config,
    connection::actions::{
        connect, finish_connecting, finish_tls_handshake, read_dns_response, read_message,
        read_upgrade_response, start_resolving, write_dns_request, write_message,
        write_upgrade_request,
    },
};
use core::net::{Ipv4Addr, SocketAddr};
use mini_sansio_dns::{Dns, DnsWants, MAX_DNS_PACKET_LEN};
use mpclipboard_shared::{
    Epoch, Message, MessageReader, MessageWriter, OptionWantsExt, UpgradeRequestWriter,
    UpgradeResponseReader, Wants, prelude::*,
};
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};

mod actions;

mod error;

mod maybe_tls_stream;
use maybe_tls_stream::MaybeTlsStream;

mod std_read_write_fd;

#[derive(Debug)]
enum State {
    Connecting(u64),
    TlsHandshake(u64),
    WritingUpgradeRequest(u64, UpgradeRequestWriter),
    ReadingUpgradeResponse(u64, UpgradeResponseReader),
    Connected(MessageReader, MessageWriter),
}

#[derive(Debug)]
enum ConnectionState {
    Disconnected(Option<u64>),
    Resolving {
        fd: OwnedFd,
        dns: Dns,
        buf: [u8; MAX_DNS_PACKET_LEN],
        last_activity_at: u64,
    },
    State {
        fd: OwnedFd,
        stream: MaybeTlsStream,
        state: State,
    },
}

#[derive(Debug)]
pub struct Connection {
    epoch: Epoch,
    state: ConnectionState,
}

impl Connection {
    const FREEZE_TIME_IN_SECS: u64 = 3;

    pub(crate) const fn new() -> Self {
        Self {
            epoch: Epoch::new(),
            state: ConnectionState::Disconnected(None),
        }
    }

    const fn name(&self) -> &'static str {
        match &self.state {
            ConnectionState::Disconnected { .. } => "Disconnected",
            ConnectionState::Resolving { .. } => "Resolving",
            ConnectionState::State { state, .. } => match state {
                State::Connecting { .. } => "Connecting",
                State::TlsHandshake { .. } => "TlsHandshake",
                State::WritingUpgradeRequest { .. } => "WritingUpgradeRequest",
                State::ReadingUpgradeResponse { .. } => "ReadingUpgradeResponse",
                State::Connected { .. } => "Connected",
            },
        }
    }

    pub(crate) fn force_disconnect(&mut self, now: u64) {
        self.state = ConnectionState::Disconnected(Some(now));
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        match &self.state {
            ConnectionState::Disconnected { .. } => Connectivity::Disconnected,
            ConnectionState::State {
                state: State::Connected { .. },
                ..
            } => Connectivity::Connected,
            ConnectionState::Resolving { .. } | ConnectionState::State { .. } => {
                Connectivity::Connecting
            }
        }
    }

    pub(crate) const fn is_disconnected(&self) -> bool {
        matches!(self.state, ConnectionState::Disconnected { .. })
    }

    fn reconnect(&mut self, now: u64, config: &Config) {
        if let Ok(ip) = config.url.host().parse::<Ipv4Addr>() {
            self.connect(now, SocketAddr::from((ip, config.url.port())), config);
            return;
        }

        let mut buf = [0; MAX_DNS_PACKET_LEN];
        match start_resolving(&config.url, &mut buf) {
            Ok((fd, dns)) => {
                self.state = ConnectionState::Resolving {
                    fd,
                    dns,
                    buf,
                    last_activity_at: now,
                };
                self.epoch.bump();
            }

            Err(err) => {
                log::error!("failed to start resolving: {err}");
                self.state = ConnectionState::Disconnected(Some(now));
            }
        }
    }

    fn connect(&mut self, now: u64, addr: SocketAddr, config: &Config) {
        match connect(addr, config) {
            Ok(Done((fd, stream))) => {
                self.state = ConnectionState::State {
                    fd,
                    state: if stream.is_tls() {
                        State::TlsHandshake(now)
                    } else {
                        let writer = UpgradeRequestWriter::new(&config.upgrade_request());
                        State::WritingUpgradeRequest(now, writer)
                    },
                    stream,
                };
                self.epoch.bump();
            }

            Ok(Pending((fd, stream))) => {
                self.state = ConnectionState::State {
                    fd,
                    stream,
                    state: State::Connecting(now),
                };
                self.epoch.bump();
            }

            Err(err) => {
                log::error!("failed to connect: {err}");
                self.state = ConnectionState::Disconnected(Some(now));
            }
        }
    }

    pub(crate) fn tick(&mut self, now: u64, config: &Config) {
        let last_activity_at = match &self.state {
            ConnectionState::Disconnected(None) => {
                self.reconnect(now, config);
                return;
            }

            ConnectionState::Disconnected(Some(disconnected_at)) => {
                let seconds_passed = now
                    .checked_sub(*disconnected_at)
                    .unwrap_or_else(|| unreachable!("time goes backwards"));

                if seconds_passed > Self::FREEZE_TIME_IN_SECS {
                    self.reconnect(now, config);
                }
                return;
            }

            ConnectionState::Resolving {
                last_activity_at, ..
            } => *last_activity_at,

            ConnectionState::State { state, .. } => match state {
                State::Connecting(last_activity_at)
                | State::TlsHandshake(last_activity_at)
                | State::WritingUpgradeRequest(last_activity_at, _)
                | State::ReadingUpgradeResponse(last_activity_at, _) => *last_activity_at,

                State::Connected { .. } => return,
            },
        };

        let seconds_passed = now
            .checked_sub(last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));

        if seconds_passed > Self::FREEZE_TIME_IN_SECS {
            log::error!("Stuck in {}, disconnecting...", self.name());
            self.state = ConnectionState::Disconnected(Some(now));
        }
    }

    pub(crate) fn push(&mut self, message: &Message) -> bool {
        let ConnectionState::State { state: active, .. } = &mut self.state else {
            return false;
        };
        let State::Connected(_reader, writer) = active else {
            return false;
        };
        writer.push(message);
        true
    }

    pub(crate) fn on_readable(&mut self, now: u64, config: &Config) -> Option<Message> {
        log::trace!("{self:?} reading...");

        match &mut self.state {
            ConnectionState::Disconnected { .. } => {
                unreachable!("can't read() in Disconnected state")
            }

            ConnectionState::Resolving {
                fd,
                dns,
                buf,
                last_activity_at,
            } => match read_dns_response(fd, dns, buf) {
                Ok(Done(mut addr)) => {
                    addr.set_port(config.url.port());
                    self.connect(now, addr, config);
                }
                Ok(Pending(())) => *last_activity_at = now,
                Err(err) => {
                    log::error!("failed to read DNS response: {err}");
                    self.state = ConnectionState::Disconnected(Some(now));
                }
            },

            ConnectionState::State { fd, stream, state } => match state {
                State::TlsHandshake(last_activity_at) => match finish_tls_handshake(stream, fd) {
                    Ok(Done(())) => {
                        let writer = UpgradeRequestWriter::new(&config.upgrade_request());
                        *state = State::WritingUpgradeRequest(now, writer);
                    }
                    Ok(Pending(())) => *last_activity_at = now,
                    Err(err) => {
                        log::error!("{err}");
                        self.state = ConnectionState::Disconnected(Some(now));
                    }
                },

                State::ReadingUpgradeResponse(last_activity_at, reader) => {
                    match read_upgrade_response(fd, stream, reader) {
                        Ok(Done(mut reader)) => {
                            let message = read_message(&mut reader, stream, fd);
                            *state = State::Connected(reader, MessageWriter::new());

                            match message {
                                Ok(Done(message)) => return Some(message),
                                Ok(Pending(())) => {}
                                Err(err) => {
                                    log::error!("failed to read message: {err}");
                                    self.state = ConnectionState::Disconnected(Some(now));
                                }
                            }
                        }
                        Ok(Pending(())) => *last_activity_at = now,
                        Err(err) => {
                            log::error!("failed to read upgrade response: {err}");
                            self.state = ConnectionState::Disconnected(Some(now));
                        }
                    }
                }

                State::Connected(reader, _writer) => match read_message(reader, stream, fd) {
                    Ok(Done(message)) => return Some(message),
                    Ok(Pending(())) => {}
                    Err(err) => {
                        log::error!("failed to read message: {err}");
                        self.state = ConnectionState::Disconnected(Some(now));
                    }
                },

                State::Connecting { .. } => {
                    unreachable!("can't read() in Connecting state")
                }

                State::WritingUpgradeRequest { .. } => {
                    unreachable!("can't read() in WritingUpgradeRequest state")
                }
            },
        }

        None
    }

    pub(crate) fn on_writable(&mut self, now: u64, config: &Config) {
        log::trace!("{self:?} writing...");

        match &mut self.state {
            ConnectionState::Disconnected { .. } => {
                unreachable!("can't write() in Disconencted state")
            }

            ConnectionState::Resolving {
                fd,
                dns,
                buf,
                last_activity_at,
            } => match write_dns_request(fd, dns, buf) {
                Ok(()) => *last_activity_at = now,
                Err(err) => {
                    log::error!("failed to write DNS request: {err}");
                    self.state = ConnectionState::Disconnected(Some(now));
                }
            },

            ConnectionState::State {
                fd,
                stream,
                state: active,
            } => match active {
                State::Connecting { .. } => {
                    if let Err(err) = finish_connecting(fd) {
                        log::error!("failed to finish connecting: {err:?}");
                        self.state = ConnectionState::Disconnected(Some(now));
                    } else if stream.is_tls() {
                        *active = State::TlsHandshake(now);
                    } else {
                        let writer = UpgradeRequestWriter::new(&config.upgrade_request());
                        *active = State::WritingUpgradeRequest(now, writer);
                    }
                }

                State::TlsHandshake(last_activity_at) => match finish_tls_handshake(stream, fd) {
                    Ok(Done(())) => {
                        let writer = UpgradeRequestWriter::new(&config.upgrade_request());
                        *active = State::WritingUpgradeRequest(now, writer);
                    }
                    Ok(Pending(())) => *last_activity_at = now,
                    Err(err) => {
                        log::error!("{err}");
                        self.state = ConnectionState::Disconnected(Some(now));
                    }
                },

                State::WritingUpgradeRequest(last_activity_at, writer) => {
                    match write_upgrade_request(fd, stream, writer) {
                        Ok(Done(())) => {
                            *active =
                                State::ReadingUpgradeResponse(now, UpgradeResponseReader::new());
                        }
                        Ok(Pending(())) => *last_activity_at = now,
                        Err(err) => {
                            log::error!("failed to write upgrade request: {err}");
                            self.state = ConnectionState::Disconnected(Some(now));
                        }
                    }
                }

                State::ReadingUpgradeResponse(last_activity_at, _reader) => {
                    match stream.flush(fd) {
                        Ok(()) => *last_activity_at = now,
                        Err(err) => {
                            log::error!("failed to flush TLS data: {err:?}");
                            self.state = ConnectionState::Disconnected(Some(now));
                        }
                    }
                }

                State::Connected(_reader, writer) => match write_message(writer, stream, fd) {
                    Ok(Done(()) | Pending(())) => {}
                    Err(err) => {
                        log::error!("failed to write message: {err}");
                        self.state = ConnectionState::Disconnected(Some(now));
                    }
                },
            },
        }
    }

    pub(crate) fn wants(&mut self) -> Option<(BorrowedFd<'_>, Epoch, Wants)> {
        match &mut self.state {
            ConnectionState::Disconnected { .. } => None,
            ConnectionState::Resolving { fd, dns, buf, .. } => {
                let wants = match dns.wants(buf) {
                    Ok(Some(DnsWants::Write { .. })) => Wants::Write,
                    Ok(Some(DnsWants::Read { .. })) => Wants::Read,
                    Ok(None) => unreachable!("Resolving never holds a finished Dns"),
                    Err(err) => unreachable!("Dns::wants() failed: {err}"),
                };
                Some(((*fd).as_fd(), self.epoch, wants))
            }
            ConnectionState::State { state, fd, stream } => {
                let fd = (*fd).as_fd();
                let wants = match state {
                    State::Connecting { .. } => Wants::Write,
                    State::TlsHandshake { .. } => stream
                        .tls_wants_read()
                        .merge_opt(stream.tls_wants_write())
                        .unwrap_or_else(|| unreachable!("TlsStream always wants soemthing")),
                    State::WritingUpgradeRequest { .. } => {
                        Wants::Write.merge_opt(stream.tls_wants_write())
                    }
                    State::ReadingUpgradeResponse { .. } => {
                        Wants::Read.merge_opt(stream.tls_wants_write())
                    }
                    State::Connected(_reader, writer) => Wants::Read
                        .merge_opt(writer.wants())
                        .merge_opt(stream.tls_wants_write()),
                };
                Some((fd, self.epoch, wants))
            }
        }
    }
}
