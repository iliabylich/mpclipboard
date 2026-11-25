use core::{net::SocketAddr, time::Duration};

use crate::{
    Context, TcpListener, TcpStream,
    client::Client,
    constants::UPGRADE_TIMEOUT_IN_SECS,
    errors::ClientError,
    pubsub::{PubSub, PubSubHandle},
    upgrade::{UpgradeReader, UpgradeWriter},
};
use externally_driven_async_rt::MaybeTimedOut;
use mpclipboard_shared::Token;

pub(crate) struct Acceptor {
    address: SocketAddr,
    raw_address: &'static str,
    token: Token,
    pubsub: PubSub,
}

impl Acceptor {
    pub(crate) fn new(address: SocketAddr, raw_address: &'static str, token: Token) -> Self {
        Self {
            address,
            raw_address,
            token,
            pubsub: PubSub::new(),
        }
    }

    pub(crate) async fn run<'a>(&'a mut self, ctx: Context<'a>) {
        let Self {
            address,
            raw_address,
            token,
            pubsub,
        } = self;
        let token: &'a Token = token;
        let pubsub: &'a PubSub = pubsub;

        info!("Trying to bind on {raw_address}");
        let mut listener = match TcpListener::bind(&ctx, *address) {
            Ok(listener) => listener,
            Err(err) => {
                error!("failed to bind {raw_address}: {err}");
                return;
            }
        };
        info!("listening on {raw_address}");

        loop {
            let Ok(task) = ctx.reserve_task().await else {
                unreachable!("the acceptor is the only task that waits for a free slot");
            };
            let stream = loop {
                match listener.accept().await {
                    Ok((stream, _)) => break stream,
                    Err(err) => error!("accept() failed: {err}"),
                }
            };
            trace!("accepted");
            let Some(pubsub) = pubsub.subscribe() else {
                unreachable!("every reserved task has a free pubsub slot");
            };
            drop(task.spawn(async move {
                if let Err(err) = serve(&ctx, stream, token, pubsub).await {
                    error!("{err}");
                }
            }));
        }
    }
}

async fn serve<'a>(
    ctx: &Context<'_>,
    stream: TcpStream<'a>,
    token: &'a Token,
    pubsub: PubSubHandle<'a>,
) -> Result<(), ClientError> {
    let result = ctx
        .timeout(
            Duration::from_secs(UPGRADE_TIMEOUT_IN_SECS),
            upgrade(stream, token, pubsub),
        )
        .await
        .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));

    let client = match result {
        MaybeTimedOut::Timeout => return Err(ClientError::UpgradeTimeout),
        MaybeTimedOut::Completed(client) => client?,
    };

    client.read_write_messages().await
}

async fn upgrade<'a>(
    mut stream: TcpStream<'a>,
    token: &Token,
    pubsub: PubSubHandle<'a>,
) -> Result<Client<'a>, ClientError> {
    let (rx, tx) = stream.split();
    let mut reader = UpgradeReader::new(rx);
    let (received, id) = reader.read().await?;

    if *token != received {
        let _ = UpgradeWriter::unauthorized(tx).write().await;
        return Err(ClientError::InvalidToken(id));
    }

    UpgradeWriter::switching_protocols(tx)
        .write()
        .await
        .map_err(|err| ClientError::UpgradeWrite { id, err })?;
    info!("[{id}] connected");

    Ok(Client::new(stream, id, pubsub))
}
