use crate::{
    ServerContext,
    constants::HANDSHAKE_TIMEOUT,
    errors::ClientError,
    pubsub::{PubSub, PubSubHandle},
    stranger::Stranger,
};
use externally_driven_async_rt::net::{TcpListener, TcpStream};
use mpclipboard_shared2::Token;

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
            let task = ctx.reserve_task().await;
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
            let _ = task.spawn(async move {
                if let Err(err) = serve(&ctx, stream, token, pubsub).await {
                    error!("{err}");
                }
            });
        }
    }
}

async fn serve<'a>(
    ctx: &ServerContext<'_>,
    stream: TcpStream,
    token: &'a Token,
    pubsub: PubSubHandle<'a>,
) -> Result<(), ClientError> {
    let stranger = Stranger::new(stream, token, pubsub);
    let Some(client) = stranger.handshake(ctx, HANDSHAKE_TIMEOUT).await? else {
        return Ok(());
    };
    client.read_write_messages(ctx).await
}
