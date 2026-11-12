use crate::tls_stream::{
    TlsError, incoming::Incoming, outgoing::Outgoing, plaintext::Plaintext, step::Step,
};
use rustls::{
    client::UnbufferedClientConnection,
    unbuffered::{AppDataRecord, ConnectionState, UnbufferedStatus},
};

pub(super) struct TlsState {
    conn: UnbufferedClientConnection,
    pub(super) incoming: Incoming,
    pub(super) outgoing: Outgoing,
    pub(super) plaintext: Plaintext,
    pub(super) peer_closed: bool,
}

impl TlsState {
    pub(super) fn new(conn: UnbufferedClientConnection) -> Self {
        Self {
            conn,
            incoming: Incoming::new(),
            outgoing: Outgoing::new(),
            plaintext: Plaintext::new(),
            peer_closed: false,
        }
    }

    pub(super) fn step(&mut self) -> Result<Step, TlsError> {
        loop {
            let UnbufferedStatus { mut discard, state } =
                self.conn.process_tls_records(self.incoming.filled_mut());

            let step = match state? {
                ConnectionState::ReadTraffic(mut traffic) => {
                    while let Some(record) = traffic.next_record() {
                        let AppDataRecord {
                            discard: record_discard,
                            payload,
                        } = record?;
                        discard += record_discard;
                        self.plaintext.push(payload)?;
                    }
                    Some(Step::Plaintext)
                }
                ConnectionState::EncodeTlsData(mut data) => {
                    let len = data
                        .encode(self.outgoing.spare_mut())
                        .map_err(TlsError::Encode)?;
                    self.outgoing.filled(len);
                    None
                }
                ConnectionState::TransmitTlsData(data) => {
                    if self.outgoing.pending().is_empty() {
                        data.done();
                        None
                    } else {
                        Some(Step::Transmit)
                    }
                }
                ConnectionState::BlockedHandshake => Some(Step::Blocked),
                ConnectionState::WriteTraffic(_) => Some(Step::Writable),
                ConnectionState::PeerClosed => Some(Step::PeerClosed),
                ConnectionState::Closed => return Err(TlsError::Closed),
                state => return Err(TlsError::UnexpectedState(format!("{state:?}"))),
            };

            self.incoming.discard(discard);

            if let Some(step) = step {
                return Ok(step);
            }
        }
    }

    pub(super) fn encrypt(&mut self, buf: &[u8]) -> Result<usize, TlsError> {
        let UnbufferedStatus { discard, state } =
            self.conn.process_tls_records(self.incoming.filled_mut());

        let len = match state? {
            ConnectionState::WriteTraffic(mut traffic) => traffic
                .encrypt(buf, self.outgoing.spare_mut())
                .map_err(TlsError::Encrypt)?,
            state => return Err(TlsError::UnexpectedState(format!("{state:?}"))),
        };
        self.outgoing.filled(len);

        self.incoming.discard(discard);

        Ok(buf.len())
    }
}
