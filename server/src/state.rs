use crate::{
    auth::Auth,
    client::{ClientRx, ClientTx, IncomingMessage, OutgoingMessage},
    store::Store,
};
use clip::Clip;
use futures_util::{SinkExt as _, Stream, StreamExt as _};
use std::{
    collections::HashMap,
    net::SocketAddr,
    pin::{Pin, pin},
    task::{Context, Poll},
};
use tokio::net::TcpStream;
use tokio_tungstenite::accept_hdr_async;

pub(crate) struct State {
    token: String,
    name_to_tx: HashMap<String, ClientTx>,
    name_to_rx: HashMap<String, ClientRx>,
    store: Store,
}

impl State {
    pub(crate) fn new(token: String) -> Self {
        Self {
            token,
            name_to_tx: HashMap::new(),
            name_to_rx: HashMap::new(),
            store: Store::empty(),
        }
    }

    pub(crate) async fn register(&mut self, stream: TcpStream, remote_addr: SocketAddr) {
        log::info!("new connection from {remote_addr:?}");

        let mut auth = Auth::new(self.token.clone());

        let ws = match accept_hdr_async(stream, &mut auth).await {
            Ok(ws) => ws,
            Err(err) => {
                log::error!("{err:?}");
                return;
            }
        };

        let name = auth.into_name();

        let (tx, rx) = ws.split();

        let mut tx = ClientTx::new(tx);
        let rx = ClientRx::new(name.clone(), rx);

        if let Some(clip) = self.store.current()
            && let Err(err) = tx.send(OutgoingMessage::Clip(clip)).await
        {
            log::error!(target: &name, "immediately crashed: {err:?}");
            return;
        }

        self.name_to_tx.insert(name.clone(), tx);
        self.name_to_rx.insert(name.clone(), rx);

        log::info!(target: &name, "starting");
    }

    pub(crate) async fn broadcast(&mut self, received_from: String, clip: Clip) {
        log::warn!("broadcasting {clip:?}");

        let mut failed = vec![];

        for (name, tx) in &mut self.name_to_tx {
            if name != &received_from
                && let Err(err) = tx.send(OutgoingMessage::Clip(clip.clone())).await
            {
                log::error!(target: name, "{err:?}");
                failed.push(name.clone());
            }
        }

        for name in failed {
            self.name_to_tx.remove(&name);
            self.name_to_rx.remove(&name);
        }
    }

    pub(crate) async fn ping(&mut self) {
        log::info!(
            "tx: {} | rx = {}",
            self.name_to_tx.len(),
            self.name_to_rx.len()
        );

        let mut failed = vec![];

        for (name, tx) in &mut self.name_to_tx {
            log::info!(target: name, "sending ping");
            if let Err(err) = tx.send(OutgoingMessage::Ping).await {
                log::error!(target: name, "failed to send ping: {err:?}");
                failed.push(name.clone());
            }
        }

        for name in failed {
            self.name_to_tx.remove(&name);
            self.name_to_rx.remove(&name);
        }
    }
}

impl Stream for State {
    type Item = (String, Clip);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::{Pending, Ready};

        let mut newest = (String::new(), Clip::zero());

        for (name, rx) in &mut self.name_to_rx {
            let rx = pin!(rx);

            match rx.poll_next(cx) {
                Ready(Some(IncomingMessage::Clip(clip))) => {
                    if clip.newer_than(&newest.1) {
                        newest = (name.clone(), clip);
                    }
                }
                Ready(Some(IncomingMessage::Error(err))) => {
                    log::error!(target: name, "error: {err:?}");
                }
                Ready(None) => {
                    log::error!(target: name, "closed");
                }
                Ready(Some(IncomingMessage::Skip)) | Pending => {}
            }
        }

        if newest.1.timestamp == 0 {
            return Pending;
        }

        if self.store.add(&newest.1) {
            Ready(Some(newest))
        } else {
            Pending
        }
    }
}
