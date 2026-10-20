use crate::{
    Connectivity,
    connection::{
        http3_connection::{Http3Connection, Http3Event},
        state::{FREEZE_TIME_IN_SECS, HEARTBEAT_INTERVAL_IN_SECS, State, http3::Http3StateError},
    },
};
use core::num::NonZeroUsize;
use mpclipboard_shared::{Message, MessageReader, MessageWriter, Now, Past, prelude::*};

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

impl Http3State {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Handshaking { .. } => "Handshaking",
            Self::AwaitingResponse { .. } => "AwaitingResponse",
            Self::Streaming { .. } => "Streaming",
        }
    }

    pub(crate) const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Handshaking { .. } | Self::AwaitingResponse { .. } => Connectivity::Connecting,
            Self::Streaming { .. } => Connectivity::Connected,
        }
    }

    pub(crate) fn is_stuck(&self, now: Now) -> bool {
        let since = match self {
            Self::Handshaking { since } | Self::AwaitingResponse { since } => *since,
            Self::Streaming { .. } => return false,
        };

        now.seconds_since(since) > FREEZE_TIME_IN_SECS
    }

    pub(crate) fn on_tick(&mut self, now: Now) {
        let Self::Streaming {
            writer,
            last_sent_at,
            ..
        } = self
        else {
            return;
        };

        if now.seconds_since(*last_sent_at) >= HEARTBEAT_INTERVAL_IN_SECS {
            writer.push_heartbeat();
            *last_sent_at = now.as_past();
        }
    }

    pub(crate) fn push(&mut self, message: &Message, now: Now) -> bool {
        let Self::Streaming {
            writer,
            last_sent_at,
            ..
        } = self
        else {
            return false;
        };
        writer.push(message);
        *last_sent_at = now.as_past();
        true
    }

    pub(crate) fn on_event(
        &mut self,
        event: Http3Event<'_>,
        now: Now,
        message: &mut Option<Message>,
    ) -> Result<Option<State>, Http3StateError> {
        match self {
            Self::Handshaking { .. } => unreachable!("the request is sent only after Handshaking"),

            Self::AwaitingResponse { .. } => match event {
                Http3Event::Status(200) => {
                    *self = Self::Streaming {
                        reader: MessageReader::empty(),
                        writer: MessageWriter::new(),
                        last_sent_at: now.as_past(),
                    };
                    Ok(None)
                }
                Http3Event::Status(401) => {
                    log::error!("server rejected the token, not reconnecting");
                    Ok(Some(State::Stopped))
                }
                Http3Event::Status(status) => Err(Http3StateError::UnexpectedStatus(status)),
                Http3Event::Body(_) => Err(Http3StateError::DataBeforeHeaders),
                Http3Event::SendStopped(_) => Ok(None),
            },

            Self::Streaming { reader, .. } => match event {
                Http3Event::Status(_) => Ok(None),
                Http3Event::Body(body) => {
                    if let Done(m) = reader.received(body)? {
                        *message = Some(m);
                    }
                    Ok(None)
                }
                Http3Event::SendStopped(code) => Err(Http3StateError::SendStopped(code)),
            },
        }
    }

    pub(crate) fn write(
        &mut self,
        http3_conn: &mut Http3Connection,
    ) -> Result<(), Http3StateError> {
        let Self::Streaming { writer, .. } = self else {
            return Ok(());
        };
        let Some(payload) = writer.remainder() else {
            return Ok(());
        };
        if !http3_conn.enqueue_request_body(payload)? {
            return Ok(());
        }

        let len = NonZeroUsize::new(payload.len())
            .unwrap_or_else(|| unreachable!("MessageWriter never holds an empty frame"));
        writer.written(len)?;
        Ok(())
    }
}
