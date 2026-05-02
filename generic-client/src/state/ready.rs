use crate::{Context, Output, state::Connected};
use anyhow::Result;
use clip::Clip;
use std::{io::ErrorKind, net::TcpStream, os::fd::AsRawFd};
use tungstenite::{Message, stream::MaybeTlsStream};

/// cbindgen:ignore
type WebSocket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;

pub(crate) struct Ready {
    ws: WebSocket,
    fd: i32,
}

impl Ready {
    pub(crate) const fn new(ws: WebSocket, fd: i32) -> Self {
        Self { ws, fd }
    }

    pub(crate) fn read_write(
        mut self,
        context: &mut Context,
        readable: bool,
        writable: bool,
    ) -> Result<(Connected, Option<Output>)> {
        let mut write_blocked = false;

        if writable
            && self.ws.can_write()
            && let Some(message) = context.pending_message_to_send.take()
        {
            match self.ws.write(message) {
                Ok(()) => match self.ws.flush() {
                    Ok(()) => {}
                    Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                        write_blocked = true;
                    }
                    Err(tungstenite::Error::WriteBufferFull(write_me_back)) => {
                        context.pending_message_to_send = Some(*write_me_back);
                    }
                    Err(err) => return Err(anyhow::anyhow!(err)),
                },

                Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                    write_blocked = true;
                }
                Err(tungstenite::Error::WriteBufferFull(write_me_back)) => {
                    context.pending_message_to_send = Some(*write_me_back);
                }
                Err(err) => return Err(anyhow::anyhow!(err)),
            }
        }

        let mut clip = None;

        if readable && self.ws.can_read() {
            match self.ws.read() {
                Ok(message) => {
                    log::trace!("{message:?}");

                    if let Message::Binary(bytes) = message {
                        match Clip::decode(bytes.into()) {
                            Ok(decoded) => clip = Some(decoded),
                            Err(err) => return Err(anyhow::anyhow!(err)),
                        }
                    }
                }
                Err(tungstenite::Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => {
                    log::trace!("nothing to read");
                }
                Err(err) => return Err(anyhow::anyhow!(err)),
            }
        }

        let wants_write = write_blocked || context.pending_message_to_send.is_some();
        context.event_loop.modify(self.fd, true, wants_write)?;

        let output = if let Some(clip) = clip
            && clip.newer_than(&context.last_clip)
        {
            context.last_clip = clip.clone();

            Some(Output::NewText { text: clip.text })
        } else {
            None
        };

        Ok((Connected::Ready(Self::new(self.ws, self.fd)), output))
    }
}

impl AsRawFd for Ready {
    fn as_raw_fd(&self) -> i32 {
        self.fd
    }
}
