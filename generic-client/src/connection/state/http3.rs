use crate::{
    Connectivity,
    config::Config,
    connection::{
        error::ConnectionError,
        http3_connection::{Http3Connection, Http3Event},
        quic_config::QuicConfig,
        state::{FREEZE_TIME_IN_SECS, HEARTBEAT_INTERVAL_IN_SECS, Readable, State},
    },
};
use core::{mem::MaybeUninit, net::SocketAddr, num::NonZeroUsize};
use h3wire::FieldRef;
use mpclipboard_shared::{
    Message, MessageReader, MessageWriter, Now, PROTOCOL_VERSION, Past, Wants, prelude::*,
};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    io::{Errno, FdFlags},
    net::{AddressFamily, RecvFlags, SendFlags, SocketType},
};
use std::time::Instant;

pub(crate) struct Http3 {
    fd: OwnedFd,
    conn: Http3Connection,
    recv_buf: Box<[MaybeUninit<u8>]>,
    state: Http3State,
}

pub(crate) enum Http3State {
    Handshaking {
        since: Past,
    },
    AwaitingResponse {
        since: Past,
    },
    Streaming {
        reader: MessageReader,
        writer: MessageWriter,
        last_sent_at: Past,
    },
}

impl Http3 {
    pub(crate) fn connect(
        now: Now,
        peer: SocketAddr,
        config: &Config,
        quic_config: &QuicConfig,
    ) -> Result<Self, ConnectionError> {
        let fd = rustix::net::socket(AddressFamily::INET, SocketType::DGRAM, None)
            .map_err(ConnectionError::OpenSocket)?;

        rustix::io::fcntl_setfd(&fd, FdFlags::CLOEXEC).map_err(ConnectionError::SetCloexec)?;
        rustix::io::ioctl_fionbio(&fd, true).map_err(ConnectionError::SwitchToNonBlocking)?;
        rustix::net::connect(&fd, &peer).map_err(ConnectionError::ConnectToAppServer)?;

        let conn = Http3Connection::new(peer, config.url.host(), quic_config)?;
        log::info!("connecting to {peer}");

        let max_udp_payload_size = usize::try_from(quic_config.endpoint.get_max_udp_payload_size())
            .unwrap_or_else(|_| unreachable!("usize is 64 bits on all supported targets"));
        let recv_buf = Box::new_uninit_slice(max_udp_payload_size);

        Ok(Self {
            fd,
            conn,
            recv_buf,
            state: Http3State::Handshaking {
                since: now.as_past(),
            },
        })
    }

    pub(crate) const fn name(&self) -> &'static str {
        match self.state {
            Http3State::Handshaking { .. } => "Handshaking",
            Http3State::AwaitingResponse { .. } => "AwaitingResponse",
            Http3State::Streaming { .. } => "Streaming",
        }
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        match self.state {
            Http3State::Handshaking { .. } | Http3State::AwaitingResponse { .. } => {
                Connectivity::Connecting
            }
            Http3State::Streaming { .. } => Connectivity::Connected,
        }
    }

    pub(crate) fn is_stuck(&self, now: Now) -> bool {
        let since = match self.state {
            Http3State::Handshaking { since } | Http3State::AwaitingResponse { since } => since,
            Http3State::Streaming { .. } => return false,
        };

        now.seconds_since(since) > FREEZE_TIME_IN_SECS
    }

    pub(crate) fn on_tick(&mut self, now: Now) {
        let Http3State::Streaming {
            writer,
            last_sent_at,
            ..
        } = &mut self.state
        else {
            return;
        };

        if now.seconds_since(*last_sent_at) >= HEARTBEAT_INTERVAL_IN_SECS {
            writer.push_heartbeat();
            *last_sent_at = now.as_past();
        }
    }

    pub(crate) fn on_readable(
        &mut self,
        now: Now,
        config: &Config,
    ) -> Result<Completion<Readable, ()>, ConnectionError> {
        self.recv()?;

        if matches!(self.state, Http3State::Handshaking { .. }) && !self.conn.is_handshaking() {
            self.open_request(now, config)?;
        }

        let mut message = None;
        while let Some(event) = self.conn.next_event()? {
            match &mut self.state {
                Http3State::Handshaking { .. } => {
                    unreachable!("the request is sent only after Handshaking")
                }

                Http3State::AwaitingResponse { .. } => match event {
                    Http3Event::Status(200) => {
                        self.state = Http3State::Streaming {
                            reader: MessageReader::default(),
                            writer: MessageWriter::default(),
                            last_sent_at: now.as_past(),
                        };
                    }
                    Http3Event::Status(401) => {
                        log::error!("server rejected the token, not reconnecting");
                        return Ok(Done(Readable::NextState(State::Stopped)));
                    }
                    Http3Event::Status(status) => {
                        return Err(ConnectionError::UnexpectedStatus(status));
                    }
                    Http3Event::Body(_) => return Err(ConnectionError::DataBeforeHeaders),
                    Http3Event::SendStopped(_) => {}
                },

                Http3State::Streaming { reader, .. } => match event {
                    Http3Event::Status(_) => {}
                    Http3Event::Body(body) => {
                        if let Done(m) = reader.received(body)? {
                            message = Some(m);
                        }
                    }
                    Http3Event::SendStopped(code) => {
                        return Err(ConnectionError::SendStopped(code));
                    }
                },
            }
        }

        match message {
            Some(message) => Ok(Done(Readable::Message(message))),
            None => Ok(Pending(())),
        }
    }

    pub(crate) fn process_expired_timers(&mut self) -> Result<(), ConnectionError> {
        self.conn.process_expired_timers()?;
        Ok(())
    }

    pub(crate) fn next_timer(&mut self) -> Option<Instant> {
        self.conn.next_timer()
    }

    pub(crate) fn push(&mut self, message: &Message, now: Now) -> bool {
        let Http3State::Streaming {
            writer,
            last_sent_at,
            ..
        } = &mut self.state
        else {
            return false;
        };
        writer.push(message);
        *last_sent_at = now.as_past();
        true
    }

    pub(crate) fn flush(&mut self) -> Result<(), ConnectionError> {
        if let Http3State::Streaming { writer, .. } = &mut self.state
            && let Some(payload) = writer.remainder()
            && self.conn.enqueue_request_body(payload)?
        {
            let len = NonZeroUsize::new(payload.len())
                .unwrap_or_else(|| unreachable!("MessageWriter never holds an empty frame"));
            writer.written(len)?;
        }
        self.conn.flush()?;

        while let Some(datagram) = self.conn.next_datagram_to_send() {
            match rustix::net::send(&self.fd, datagram, SendFlags::empty()) {
                Ok(_) => {}
                Err(Errno::AGAIN) => break,
                Err(errno) => {
                    log::warn!("failed to send(): {errno:?}");
                    break;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn wants(&self) -> (BorrowedFd<'_>, Wants) {
        (self.fd.as_fd(), Wants::Read)
    }

    fn recv(&mut self) -> Result<(), ConnectionError> {
        loop {
            let datagram =
                match rustix::net::recv(&self.fd, &mut *self.recv_buf, RecvFlags::empty()) {
                    Ok(((received, _rest), _len)) => received,
                    Err(Errno::AGAIN) => return Ok(()),
                    Err(errno) => {
                        log::warn!("failed to recv(): {errno:?}");
                        return Ok(());
                    }
                };
            self.conn.process_incoming_datagram(datagram)?;
        }
    }

    fn open_request(&mut self, now: Now, config: &Config) -> Result<(), ConnectionError> {
        let authority = config.url.header();
        let headers = [
            FieldRef::new(b":method", b"POST"),
            FieldRef::new(b":scheme", b"https"),
            FieldRef::new(b":authority", authority.as_bytes()),
            FieldRef::new(b":path", b"/"),
            FieldRef::new(b"token", config.token.as_bytes()),
            FieldRef::new(b"id", config.id.as_bytes()),
            FieldRef::new(b"version", PROTOCOL_VERSION.as_bytes()),
        ];
        self.conn.open_request_and_enqueue_headers(&headers)?;

        self.state = Http3State::AwaitingResponse {
            since: now.as_past(),
        };
        Ok(())
    }
}

impl core::fmt::Debug for Http3 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Http3")
            .field("state", &self.name())
            .finish_non_exhaustive()
    }
}
