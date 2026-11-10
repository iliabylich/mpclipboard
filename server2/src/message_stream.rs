use crate::{
    ServerContext, errors::ClientError, id::ID, message::Message, message_reader::MessageReader,
};
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

    pub(crate) async fn next(
        &mut self,
        ctx: &ServerContext<'_>,
    ) -> Result<Option<Message>, ClientError> {
        let id = self.id;
        loop {
            let len = match self.rx.read(ctx, &mut self.buf).await {
                Ok(0) => {
                    info!("[{id}] disconnected");
                    return Ok(None);
                }
                Ok(len) => len,
                Err(err) => return Err(ClientError::Read { id, err }),
            };

            let message = self.reader.received(&self.buf[..len]);
            let message = message.map_err(|err| ClientError::MalformedFrame { id, err })?;
            if let Some(message) = message {
                return Ok(Some(message));
            }
        }
    }
}
