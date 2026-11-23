use core::time::Duration;

use crate::{
    ServerContext,
    client::Client,
    constants::UPGRADE_TIMEOUT_IN_SECS,
    errors::ClientError,
    pubsub::{PubSub, PubSubHandle},
    upgrade::{UpgradeReader, UpgradeWriter},
};
use externally_driven_async_rt::{
    MaybeTimedOut,
    net::{TcpListener, TcpStream},
};
use mpclipboard_shared::Token;

pub(crate) struct Acceptor {
    listener: TcpListener,
    token: Token,
    pubsub: PubSub,
}

impl Acceptor {
    pub(crate) fn new(listener: TcpListener, token: Token) -> Self {
        Self {
            listener,
            token,
            pubsub: PubSub::new(),
        }
    }

    pub(crate) async fn run<'a>(&'a mut self, ctx: ServerContext<'a>) {
        let Self {
            listener,
            token,
            pubsub,
        } = self;
        let token: &'a Token = token;
        let pubsub: &'a PubSub = pubsub;

        loop {
            let Ok(task) = ctx.reserve_task().await else {
                unreachable!("the acceptor is the only task that waits for a free slot");
            };
            let stream = loop {
                match listener.accept(&ctx).await {
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
    ctx: &ServerContext<'_>,
    stream: TcpStream,
    token: &'a Token,
    pubsub: PubSubHandle<'a>,
) -> Result<(), ClientError> {
    let result = ctx
        .timeout(
            Duration::from_secs(UPGRADE_TIMEOUT_IN_SECS),
            upgrade(ctx, stream, token, pubsub),
        )
        .await
        .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));

    let client = match result {
        MaybeTimedOut::Timeout => return Err(ClientError::UpgradeTimeout),
        MaybeTimedOut::Completed(client) => client?,
    };

    client.read_write_messages(ctx).await
}

async fn upgrade<'a>(
    ctx: &ServerContext<'_>,
    mut stream: TcpStream,
    token: &Token,
    pubsub: PubSubHandle<'a>,
) -> Result<Client<'a>, ClientError> {
    let (rx, tx) = stream.split();
    let mut reader = UpgradeReader::new(rx);
    let (received, id) = reader.read(ctx).await?;

    if *token != received {
        let _ = UpgradeWriter::unauthorized(tx).write(ctx).await;
        return Err(ClientError::InvalidToken(id));
    }

    UpgradeWriter::switching_protocols(tx)
        .write(ctx)
        .await
        .map_err(|err| ClientError::UpgradeWrite { id, err })?;
    info!("[{id}] connected");

    Ok(Client::new(stream, id, pubsub))
}
