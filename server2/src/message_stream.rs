use crate::{ServerContext, handshake::ID, message::Message, message_reader::MessageReader};
use externally_driven_async_rt::net::BorrowedReadHalf;

pub(crate) struct MessageStream<'s> {
    rx: BorrowedReadHalf<'s>,
    buf: [u8; Message::BYTESIZE],
    reader: MessageReader,
    id: ID,
}

impl<'s> MessageStream<'s> {
    pub(crate) fn new(rx: BorrowedReadHalf<'s>, reader: MessageReader, id: ID) -> Self {
        Self {
            rx,
            buf: [0; Message::BYTESIZE],
            reader,
            id,
        }
    }

    pub(crate) async fn next(&mut self, ctx: &ServerContext<'_>) -> Result<Option<Message>, ()> {
        let id = self.id;
        loop {
            let len = match self.rx.read(ctx, &mut self.buf).await {
                Ok(0) => {
                    log::info!("[{id}] disconnected");
                    return Ok(None);
                }
                Ok(len) => len,
                Err(err) => {
                    log::error!("[{id}] read() failed: {err}");
                    return Err(());
                }
            };

            match self.reader.received(&self.buf[..len]) {
                Ok(Some(message)) => return Ok(Some(message)),
                Ok(None) => {}
                Err(()) => {
                    log::error!("[{id}] received a malformed frame");
                    return Err(());
                }
            }
        }
    }
}
