use crate::{ServerContext, id::ID, message::Message, message_reader::MessageReader};
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

    pub(crate) async fn next(&mut self, ctx: &ServerContext<'_>) -> Result<Message, ()> {
        let id = self.id;
        loop {
            let len = match self.rx.read(ctx, &mut self.buf).await {
                Ok(0) => {
                    info!("[{id}] disconnected");
                    return Err(());
                }
                Ok(len) => len,
                Err(err) => {
                    error!("[{id}] read() failed: {err}");
                    return Err(());
                }
            };

            match self.reader.received(&self.buf[..len]) {
                Ok(Some(message)) => return Ok(message),
                Ok(None) => {}
                Err(()) => {
                    error!("[{id}] received a malformed frame");
                    return Err(());
                }
            }
        }
    }
}
