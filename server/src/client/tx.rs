use bytes::Bytes;
use clip::Clip;
use futures_util::{Sink, stream::SplitSink};
use std::{
    pin::{Pin, pin},
    task::{Context, Poll},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

type StreamTx = SplitSink<WebSocketStream<TcpStream>, Message>;

pub(crate) struct ClientTx {
    tx: StreamTx,
}

impl ClientTx {
    pub(crate) fn new(tx: StreamTx) -> Self {
        Self { tx }
    }
}

pub(crate) enum OutgoingMessage {
    Ping,
    Clip(Clip),
}

impl From<OutgoingMessage> for Message {
    fn from(message: OutgoingMessage) -> Self {
        match message {
            OutgoingMessage::Ping => Self::Ping(Bytes::from_static(b"<ping>")),
            OutgoingMessage::Clip(clip) => Self::Binary(Bytes::from(clip.encode())),
        }
    }
}

impl Sink<OutgoingMessage> for ClientTx {
    type Error = tokio_tungstenite::tungstenite::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        pin!(&mut self.tx).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: OutgoingMessage) -> Result<(), Self::Error> {
        pin!(&mut self.tx).start_send(item.into())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        pin!(&mut self.tx).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        pin!(&mut self.tx).poll_close(cx)
    }
}
