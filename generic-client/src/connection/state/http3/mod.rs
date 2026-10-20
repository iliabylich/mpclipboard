mod error;
pub(crate) use error::Http3StateError;

mod http3_state;
use http3_state::Http3State;

use crate::{
    Connectivity,
    config::Config,
    connection::{
        actions::connect, http3_connection::Http3Connection, quic_config::QuicConfig,
        state::Readable,
    },
};
use core::{net::SocketAddr, time::Duration};
use h3wire::FieldRef;
use mpclipboard_shared::{Message, Now, PROTOCOL_VERSION, Wants, prelude::*};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    io::Errno,
    net::{RecvFlags, SendFlags},
};

pub(crate) struct Http3 {
    fd: OwnedFd,
    connection: Http3Connection,
    recv_buf: Vec<u8>,
    state: Http3State,
}

impl Http3 {
    pub(crate) fn connect(
        now: Now,
        peer: SocketAddr,
        config: &Config,
        quic_config: &QuicConfig,
    ) -> Result<Self, Http3StateError> {
        let fd = connect(peer)?;
        let http3_conn = Http3Connection::new(peer, config.url.host(), quic_config)?;
        log::info!("connecting to {peer}");

        let recv_buf = vec![0; usize::try_from(quic_config.endpoint.get_max_udp_payload_size())?];

        Ok(Self {
            fd,
            connection: http3_conn,
            recv_buf,
            state: Http3State::Handshaking {
                since: now.as_past(),
            },
        })
    }

    pub(crate) const fn name(&self) -> &'static str {
        self.state.name()
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        self.state.connectivity()
    }

    pub(crate) fn is_stuck(&self, now: Now) -> bool {
        self.state.is_stuck(now)
    }

    pub(crate) fn on_tick(&mut self, now: Now) {
        self.state.on_tick(now);
    }

    pub(crate) fn on_readable(
        &mut self,
        now: Now,
        config: &Config,
    ) -> Result<Completion<Readable, ()>, Http3StateError> {
        self.recv()?;

        if matches!(self.state, Http3State::Handshaking { .. }) && !self.connection.is_handshaking()
        {
            self.open_request(now, config)?;
        }

        let mut message = None;
        while let Some(event) = self.connection.next_event()? {
            if let Some(next) = self.state.on_event(event, now, &mut message)? {
                return Ok(Done(Readable::NextState(next)));
            }
        }

        match message {
            Some(message) => Ok(Done(Readable::Message(message))),
            None => Ok(Pending(())),
        }
    }

    pub(crate) fn process_expired_timers(&mut self) -> Result<(), Http3StateError> {
        self.connection.process_expired_timers()?;
        Ok(())
    }

    pub(crate) fn time_until_next_timer(&mut self) -> Option<Duration> {
        self.connection.time_until_next_timer()
    }

    pub(crate) fn push(&mut self, message: &Message, now: Now) -> bool {
        self.state.push(message, now)
    }

    pub(crate) fn flush(&mut self) -> Result<(), Http3StateError> {
        self.state.write(&mut self.connection)?;
        self.connection.flush()?;

        while let Some(datagram) = self.connection.next_datagram_to_send() {
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

    fn recv(&mut self) -> Result<(), Http3StateError> {
        loop {
            let len = match rustix::net::recv(&self.fd, &mut self.recv_buf, RecvFlags::empty()) {
                Ok((len, _)) => len,
                Err(Errno::AGAIN) => return Ok(()),
                Err(errno) => {
                    log::warn!("failed to recv(): {errno:?}");
                    return Ok(());
                }
            };
            let datagram = self
                .recv_buf
                .get(..len)
                .unwrap_or_else(|| unreachable!("recv() never writes past the buffer"));
            self.connection.process_incoming_datagram(datagram)?;
        }
    }

    fn open_request(&mut self, now: Now, config: &Config) -> Result<(), Http3StateError> {
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
        self.connection.open_request_and_enqueue_headers(&headers)?;

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
