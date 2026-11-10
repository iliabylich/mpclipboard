use crate::{MAX_CLIENTS, ServerContext, client, handshake::Token, message::Message, slot::Slot};
use core::cell::{Cell, RefCell};
use externally_driven_async_rt::net::TcpListener;

pub(crate) struct Acceptor {
    listener: TcpListener,
    token: Token,
    current: Cell<Option<Message>>,
    slots: [RefCell<Slot>; MAX_CLIENTS],
}

impl Acceptor {
    pub(crate) fn new(listener: TcpListener, token: Token) -> Self {
        Self {
            listener,
            token,
            current: Cell::new(None),
            slots: [const { RefCell::new(Slot::Free) }; MAX_CLIENTS],
        }
    }

    pub(crate) async fn run<'a>(&'a mut self, ctx: ServerContext<'a>) {
        let Self {
            listener,
            token,
            current,
            slots,
        } = self;
        let token: &'a Token = token;
        let current: &'a Cell<Option<Message>> = current;
        let slots: &'a [RefCell<Slot>] = slots;

        loop {
            let task = ctx.reserve_task().await;
            let (stream, peer) = loop {
                match listener.accept(&ctx).await {
                    Ok(accepted) => break accepted,
                    Err(err) => log::error!("accept() failed: {err}"),
                }
            };
            log::trace!("[{peer}] accepted");
            task.spawn(move |ctx| client::client(ctx, stream, peer, token, slots, current));
        }
    }
}
