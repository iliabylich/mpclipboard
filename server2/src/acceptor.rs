use crate::{ServerContext, client, handshake::Token, pubsub::PubSub};
use externally_driven_async_rt::net::TcpListener;

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
                    Err(err) => log::error!("accept() failed: {err}"),
                }
            };
            log::trace!("accepted");
            let Ok(pubsub) = pubsub.subscribe() else {
                continue;
            };
            task.spawn(move |ctx| client::client(ctx, stream, token, pubsub));
        }
    }
}
