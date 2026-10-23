mod quinn_h3wire_recv_adapter;
use quinn_h3wire_recv_adapter::{FeedH3Wire, QuinnH3WireRecvAdapter, ReadQuinn};

mod stream_id;
use stream_id::StreamId;

mod var_int_ext;
use var_int_ext::VarIntExt;

use crate::connection::{error::ConnectionError, quic_config::QuicConfig};
use alloc::{collections::VecDeque, sync::Arc};
use bytes::{Bytes, BytesMut};
use core::net::SocketAddr;
use h3wire::{Action, Event, FieldRef, H3Code, Role, UsageError};
use quinn_proto::{
    ConnectionHandle, DatagramEvent, Dir, Endpoint, StreamEvent, VarInt, WriteError,
};
use std::time::Instant;

pub(crate) struct Http3Connection {
    peer: SocketAddr,
    endpoint: Endpoint,
    handle: ConnectionHandle,
    quic_conn: quinn_proto::Connection,
    h3_conn: h3wire::Connection,
    peer_uni: Vec<QuinnH3WireRecvAdapter>,
    request: Option<(QuinnH3WireRecvAdapter, Vec<u8>)>,
    responses: VecDeque<Vec<u8>>,
    datagram: Vec<u8>,
}

#[derive(Clone, Copy)]
pub(crate) enum Http3Event<'a> {
    Status(u16),
    Body(&'a [u8]),
    SendStopped(u64),
}

impl Http3Connection {
    pub(crate) fn new(
        peer: SocketAddr,
        host: &str,
        quic_config: &QuicConfig,
    ) -> Result<Self, ConnectionError> {
        let mut endpoint = Endpoint::new(Arc::clone(&quic_config.endpoint), None, true, None);
        let (handle, quic_conn) =
            endpoint.connect(Instant::now(), quic_config.client.clone(), peer, host)?;

        Ok(Self {
            peer,
            endpoint,
            handle,
            quic_conn,
            h3_conn: h3wire::Connection::new(Role::Client, h3wire::Config::default()),
            peer_uni: vec![],
            request: None,
            responses: VecDeque::new(),
            datagram: vec![],
        })
    }

    pub(crate) fn is_handshaking(&self) -> bool {
        self.quic_conn.is_handshaking()
    }

    pub(crate) fn process_incoming_datagram(
        &mut self,
        datagram: &[u8],
    ) -> Result<(), ConnectionError> {
        let mut response = vec![];
        match self.endpoint.handle(
            Instant::now(),
            self.peer,
            None,
            None,
            BytesMut::from(datagram),
            &mut response,
        ) {
            Some(DatagramEvent::ConnectionEvent(_, event)) => self.quic_conn.handle_event(event),
            Some(DatagramEvent::Response(transmit)) => {
                response.truncate(transmit.size);
                self.responses.push_back(response);
            }
            Some(DatagramEvent::NewConnection(_)) | None => {}
        }

        self.process_quinn_events_and_forward_to_h3wire()?;
        self.accept_new_server_uni_streams();
        self.process_quinn_uni_streams_and_forward_to_h3wire()?;
        self.process_quinn_request_stream()
    }

    pub(crate) fn process_expired_timers(&mut self) -> Result<(), ConnectionError> {
        self.quic_conn.handle_timeout(Instant::now());
        self.process_quinn_events_and_forward_to_h3wire()
    }

    pub(crate) fn next_timer(&mut self) -> Option<Instant> {
        self.quic_conn.poll_timeout()
    }

    pub(crate) fn open_request_and_enqueue_headers(
        &mut self,
        headers: &[FieldRef<'_>],
    ) -> Result<(), ConnectionError> {
        if self.request.is_some() {
            return Err(ConnectionError::RequestAlreadySent);
        }

        let id = open_quinn_stream(&mut self.quic_conn, Dir::Bi)?;
        self.h3_conn.send_headers(id.into(), headers, false)?;

        self.request = Some((QuinnH3WireRecvAdapter::new(id), vec![]));
        Ok(())
    }

    pub(crate) fn enqueue_request_body(&mut self, body: &[u8]) -> Result<bool, ConnectionError> {
        let (recv, outgoing) = self.request.as_mut().ok_or(ConnectionError::NoRequest)?;
        if !outgoing.is_empty() {
            return Ok(false);
        }

        let stream = recv.id.into();
        let frame = match self.h3_conn.send_data(
            stream,
            u64::try_from(body.len())
                .unwrap_or_else(|_| unreachable!("usize is never wider than u64")),
            false,
        ) {
            Ok(frame) => frame,
            Err(UsageError::Blocked) => return Ok(false),
            Err(err) => return Err(ConnectionError::H3Usage(err)),
        };
        outgoing.extend_from_slice(frame.prefix());
        outgoing.extend_from_slice(body);
        Ok(true)
    }

    pub(crate) fn next_event(&mut self) -> Result<Option<Http3Event<'_>>, ConnectionError> {
        let Some((recv, _)) = &mut self.request else {
            return Ok(None);
        };

        loop {
            if let Some(event) = next_h3wire_event(&mut self.h3_conn)? {
                return Ok(Some(event));
            }

            match recv.feed_h3wire(&mut self.h3_conn)? {
                FeedH3Wire::Nothing => return Ok(None),
                FeedH3Wire::Progress => {}
                FeedH3Wire::Body(range) => {
                    return Ok(Some(Http3Event::Body(recv.body(range))));
                }
            }
        }
    }

    pub(crate) fn flush(&mut self) -> Result<(), ConnectionError> {
        if !self.quic_conn.is_handshaking() {
            loop {
                let mut moved = self.process_h3wire_actions_and_forward_to_quinn()?;
                moved |= self.copy_h3wire_bytes_to_quinn()?;
                moved |= self.copy_request_body_to_quinn()?;
                if !moved {
                    break;
                }
            }
        }

        while let Some(event) = self.quic_conn.poll_endpoint_events() {
            if let Some(event) = self.endpoint.handle_event(self.handle, event) {
                self.quic_conn.handle_event(event);
            }
        }
        Ok(())
    }

    pub(crate) fn next_datagram_to_send(&mut self) -> Option<&[u8]> {
        if let Some(response) = self.responses.pop_front() {
            self.datagram = response;
            return Some(&self.datagram);
        }

        self.datagram.clear();
        let transmit = self
            .quic_conn
            .poll_transmit(Instant::now(), 1, &mut self.datagram)?;
        let datagram = self
            .datagram
            .get(..transmit.size)
            .unwrap_or_else(|| unreachable!("quinn never reports more than it wrote"));
        Some(datagram)
    }

    fn process_quinn_events_and_forward_to_h3wire(&mut self) -> Result<(), ConnectionError> {
        while let Some(event) = self.quic_conn.poll() {
            match event {
                quinn_proto::Event::ConnectionLost {
                    reason: quinn_proto::ConnectionError::TimedOut,
                } => return Err(ConnectionError::IdleTimeout),
                quinn_proto::Event::ConnectionLost { reason } => {
                    return Err(ConnectionError::ConnectionLost(reason));
                }
                quinn_proto::Event::Stream(StreamEvent::Stopped { id, error_code }) => {
                    self.h3_conn.stop_sending_received(
                        StreamId::from(id).into(),
                        H3Code(error_code.into_inner()),
                    )?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn accept_new_server_uni_streams(&mut self) {
        while let Some(id) = self.quic_conn.streams().accept(Dir::Uni) {
            self.peer_uni.push(QuinnH3WireRecvAdapter::new(id.into()));
        }
    }

    fn process_quinn_uni_streams_and_forward_to_h3wire(&mut self) -> Result<(), ConnectionError> {
        for adapter in &mut self.peer_uni {
            if let ReadQuinn::Reset(code) = adapter.read_quinn(&mut self.quic_conn)? {
                self.h3_conn
                    .stream_reset_received(adapter.id.into(), code)?;
            }
            while !matches!(adapter.feed_h3wire(&mut self.h3_conn)?, FeedH3Wire::Nothing) {}
        }
        self.peer_uni.retain(|adapter| !adapter.done);
        Ok(())
    }

    fn process_quinn_request_stream(&mut self) -> Result<(), ConnectionError> {
        if let Some((recv, _)) = &mut self.request
            && let ReadQuinn::Reset(code) = recv.read_quinn(&mut self.quic_conn)?
        {
            self.h3_conn.stream_reset_received(recv.id.into(), code)?;
        }
        Ok(())
    }

    fn process_h3wire_actions_and_forward_to_quinn(&mut self) -> Result<bool, ConnectionError> {
        let mut moved = false;
        while let Some(action) = self.h3_conn.poll_action() {
            moved = true;
            match action {
                Action::OpenUni(kind) => {
                    let id = open_quinn_stream(&mut self.quic_conn, Dir::Uni)?;
                    self.h3_conn.bind_uni(kind, id.into())?;
                }
                Action::ResetStream { stream, code } => {
                    let _ = self
                        .quic_conn
                        .send_stream(StreamId::from(stream).into())
                        .reset(VarInt::from_h3_code(code));
                }
                Action::StopSending { stream, code } => {
                    let _ = self
                        .quic_conn
                        .recv_stream(StreamId::from(stream).into())
                        .stop(VarInt::from_h3_code(code));
                }
                Action::FinishStream(_) => unreachable!("the request body never ends"),
                Action::CloseConnection { code, reason } => {
                    self.quic_conn.close(
                        Instant::now(),
                        VarInt::from_h3_code(code),
                        Bytes::from_static(reason.as_bytes()),
                    );
                    return Err(ConnectionError::Closed(code.0));
                }
            }
        }
        Ok(moved)
    }

    fn copy_h3wire_bytes_to_quinn(&mut self) -> Result<bool, ConnectionError> {
        let mut moved = false;
        let streams = self.h3_conn.sendable().collect::<Vec<_>>();
        for stream in streams {
            let Some(bytes) = self.h3_conn.poll_send(stream) else {
                continue;
            };
            let Some(written) =
                write_quinn_stream(&mut self.quic_conn, StreamId::from(stream), bytes)?
            else {
                continue;
            };
            self.h3_conn.sent(stream, written)?;
            moved |= written > 0;
        }
        Ok(moved)
    }

    fn copy_request_body_to_quinn(&mut self) -> Result<bool, ConnectionError> {
        let Some((recv, outgoing)) = &mut self.request else {
            return Ok(false);
        };
        if outgoing.is_empty() {
            return Ok(false);
        }

        let Some(written) = write_quinn_stream(&mut self.quic_conn, recv.id, outgoing)? else {
            return Ok(false);
        };
        self.h3_conn.data_written(recv.id.into(), written)?;
        outgoing.drain(..written);
        Ok(written > 0)
    }
}

fn next_h3wire_event(
    h3_conn: &mut h3wire::Connection,
) -> Result<Option<Http3Event<'static>>, ConnectionError> {
    while let Some(event) = h3_conn.poll_event() {
        match event {
            Event::Headers { block, .. } => {
                let status = h3_conn.headers(block)?.pseudo().status;
                h3_conn.release(block);

                let status = status.ok_or(ConnectionError::MissingStatus)?;
                return Ok(Some(Http3Event::Status(status)));
            }
            Event::Finished(_) => return Err(ConnectionError::ResponseFinished),
            Event::StreamAborted { code, .. } => {
                return Err(ConnectionError::ResponseReset(code.0));
            }
            Event::SendStopped { code, .. } => {
                return Ok(Some(Http3Event::SendStopped(code.0)));
            }
            Event::GoAway { .. } => return Err(ConnectionError::GoAway),
            Event::Closed { code } => return Err(ConnectionError::Closed(code.0)),
            Event::PeerSettings | Event::UniStream { .. } => {}
        }
    }
    Ok(None)
}

fn write_quinn_stream(
    quic_conn: &mut quinn_proto::Connection,
    id: StreamId,
    bytes: &[u8],
) -> Result<Option<usize>, ConnectionError> {
    match quic_conn.send_stream(id.into()).write(bytes) {
        Ok(written) => Ok(Some(written)),
        Err(WriteError::Blocked) => Ok(None),
        Err(err) => Err(ConnectionError::Write(err)),
    }
}

fn open_quinn_stream(
    quic_conn: &mut quinn_proto::Connection,
    dir: Dir,
) -> Result<StreamId, ConnectionError> {
    let id = quic_conn
        .streams()
        .open(dir)
        .ok_or(ConnectionError::NoStreamsAvailable)?;
    Ok(StreamId::from(id))
}
