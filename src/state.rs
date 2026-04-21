use crate::{
    auth::Auth,
    client::{ClientRx, ClientTx, IncomingMessage, OutgoingMessage},
    clip::Clip,
    store::Store,
};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;
use tokio_stream::{StreamMap, StreamNotifyClose};
use tokio_tungstenite::accept_hdr_async;

pub(crate) struct State {
    token: String,
    name_to_tx: HashMap<String, ClientTx>,
    name_to_rx: StreamMap<String, StreamNotifyClose<ClientRx>>,
    store: Store,
}

impl State {
    pub(crate) fn new(token: String) -> Self {
        Self {
            token,
            name_to_tx: HashMap::new(),
            name_to_rx: StreamMap::new(),
            store: Store::empty(),
        }
    }

    pub(crate) async fn register(&mut self, stream: TcpStream, remote_addr: SocketAddr) {
        log::info!("new connection from {remote_addr:?}");

        let mut auth = Auth::new(self.token.to_string());

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
        let rx = StreamNotifyClose::new(ClientRx::new(name.clone(), rx));

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

    pub(crate) async fn recv(&mut self) -> Option<(String, Clip)> {
        match self.name_to_rx.next().await? {
            (name, None) => {
                log::info!(target: &name, "dead stream");
                self.name_to_tx.remove(&name);
                None
            }

            (_, Some(IncomingMessage::Skip)) => None,
            (name, Some(IncomingMessage::Error(err))) => {
                log::error!(target: &name, "{err:?}");
                self.name_to_tx.remove(&name);
                None
            }

            (name, Some(IncomingMessage::Clip(clip))) => {
                if self.store.add(&clip) {
                    Some((name, clip))
                } else {
                    None
                }
            }
        }
    }

    pub(crate) async fn broadcast(&mut self, received_from: String, clip: Clip) {
        log::warn!("broadcasting {clip:?}");

        let mut failed = vec![];

        for (name, tx) in self.name_to_tx.iter_mut() {
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

        for (name, tx) in self.name_to_tx.iter_mut() {
            log::info!(target: name, "sending ping");
            if let Err(err) = tx.send(OutgoingMessage::Ping).await {
                log::error!(target: name, "failed to send ping: {err:?}");
                failed.push(name.to_string());
            }
        }

        for name in failed {
            self.name_to_tx.remove(&name);
            self.name_to_rx.remove(&name);
        }
    }
}
