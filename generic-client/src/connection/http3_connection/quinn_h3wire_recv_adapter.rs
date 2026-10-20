use crate::connection::http3_connection::{Http3Error, StreamId};
use core::ops::Range;
use h3wire::{H3Code, Recv};
use quinn_proto::{ReadError, ReadableError};

pub(crate) struct QuinnH3WireRecvAdapter {
    pub(crate) id: StreamId,
    buf: Vec<u8>,
    consumed: usize,
    fin: bool,
    pub(crate) done: bool,
}

impl QuinnH3WireRecvAdapter {
    pub(crate) const fn new(id: StreamId) -> Self {
        Self {
            id,
            buf: vec![],
            consumed: 0,
            fin: false,
            done: false,
        }
    }

    pub(crate) fn read_quinn(
        &mut self,
        quic_conn: &mut quinn_proto::Connection,
    ) -> Result<ReadQuinn, Http3Error> {
        if self.fin || self.done {
            return Ok(ReadQuinn::Open);
        }

        let mut stream = quic_conn.recv_stream(self.id.into());
        let mut chunks = match stream.read(true) {
            Ok(chunks) => chunks,
            Err(ReadableError::ClosedStream) => return Ok(ReadQuinn::Open),
            Err(err) => return Err(Http3Error::Readable(err)),
        };

        let mut reset = None;
        loop {
            match chunks.next(usize::MAX) {
                Ok(Some(chunk)) => self.buf.extend_from_slice(&chunk.bytes),
                Ok(None) => {
                    self.fin = true;
                    break;
                }
                Err(ReadError::Blocked) => break,
                Err(ReadError::Reset(code)) => {
                    reset = Some(code);
                    break;
                }
            }
        }
        let _ = chunks.finalize();

        let Some(code) = reset else {
            return Ok(ReadQuinn::Open);
        };
        self.done = true;
        Ok(ReadQuinn::Reset(H3Code(code.into_inner())))
    }

    pub(crate) fn feed_h3wire(
        &mut self,
        h3_conn: &mut h3wire::Connection,
    ) -> Result<FeedH3Wire, Http3Error> {
        self.buf.drain(..self.consumed);
        self.consumed = 0;

        if self.done || (self.buf.is_empty() && !self.fin) {
            return Ok(FeedH3Wire::Nothing);
        }

        let (consumed, fed) = match h3_conn.recv(self.id.into(), &self.buf, self.fin)? {
            Recv::Paused => return Ok(FeedH3Wire::Nothing),
            Recv::Body { consumed, range } => (consumed, FeedH3Wire::Body(range)),
            Recv::Consumed(consumed)
            | Recv::Frame { consumed, .. }
            | Recv::Raw { consumed, .. } => (consumed, FeedH3Wire::Progress),
        };

        self.done = self.fin && consumed == self.buf.len();
        self.consumed = consumed;
        Ok(fed)
    }

    pub(crate) fn body(&self, range: Range<usize>) -> &[u8] {
        self.buf
            .get(range)
            .unwrap_or_else(|| unreachable!("h3wire ranges index the given buffer"))
    }
}

#[must_use]
pub(crate) enum ReadQuinn {
    Open,
    Reset(H3Code),
}

pub(crate) enum FeedH3Wire {
    Nothing,
    Progress,
    Body(Range<usize>),
}
