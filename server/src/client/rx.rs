use clip::Clip;
use futures_util::{Stream, StreamExt, stream::SplitStream};
use std::{
    pin::{Pin, pin},
    task::ready,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

type StreamRx = SplitStream<WebSocketStream<TcpStream>>;

pub(crate) struct ClientRx {
    name: String,
    rx: StreamRx,
}

impl ClientRx {
    pub(crate) const fn new(name: String, rx: StreamRx) -> Self {
        Self { name, rx }
    }
}

pub(crate) enum IncomingMessage {
    Clip(Clip),
    Skip,
    Error(anyhow::Error),
}

impl Stream for ClientRx {
    type Item = IncomingMessage;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll::Ready;

        let mut rx = pin!(&mut self.rx);

        let Some(message) = ready!(rx.poll_next_unpin(cx)) else {
            return Ready(None);
        };

        let message = match message {
            Ok(message) => message,
            Err(err) => {
                use tokio_tungstenite::tungstenite::{Error::Protocol, error::ProtocolError};

                if matches!(err, Protocol(ProtocolError::ResetWithoutClosingHandshake)) {
                    return Ready(Some(IncomingMessage::Error(anyhow::anyhow!(
                        "disconnected"
                    ))));
                }

                return Ready(Some(IncomingMessage::Error(anyhow::anyhow!(err))));
            }
        };

        let bytes = match message {
            Message::Text(_) => {
                log::info!(target: &self.name, "received (unsupported) text");
                return Ready(Some(IncomingMessage::Skip));
            }
            Message::Binary(bytes) => bytes.to_vec(),
            Message::Ping(_) => {
                log::info!(target: &self.name, "received ping");
                return Ready(Some(IncomingMessage::Skip));
            }
            Message::Pong(_) => {
                log::info!(target: &self.name, "received pong");
                return Ready(Some(IncomingMessage::Skip));
            }
            Message::Close(_) => {
                log::info!(target: &self.name, "received close frame");
                return Ready(Some(IncomingMessage::Skip));
            }
            Message::Frame(_) => {
                log::info!(target: &self.name, "received frame");
                return Ready(Some(IncomingMessage::Skip));
            }
        };

        match Clip::decode(bytes) {
            Ok(clip) => Ready(Some(IncomingMessage::Clip(clip))),
            Err(err) => Ready(Some(IncomingMessage::Error(anyhow::anyhow!(err)))),
        }
    }
}
