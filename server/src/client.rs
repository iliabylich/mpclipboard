use crate::{ServerContext, errors::ClientError, pubsub::PubSubHandle};
use externally_driven_async_rt::{Either3, net::TcpStream, select3};
use mpclipboard_shared::{ID, MessageReader, MessageWriter};

pub(crate) struct Client<'a> {
    stream: TcpStream,
    id: ID,
    pubsub: PubSubHandle<'a>,
}

impl<'a> Client<'a> {
    pub(crate) fn new(stream: TcpStream, id: ID, pubsub: PubSubHandle<'a>) -> Self {
        Self { stream, id, pubsub }
    }

    pub(crate) async fn read_write_messages(
        self,
        ctx: &ServerContext<'_>,
    ) -> Result<(), ClientError> {
        let Self {
            mut stream,
            id,
            pubsub,
        } = self;

        let (rx, tx) = stream.split();
        let mut reader = MessageReader::new(rx);
        let mut writer = MessageWriter::new(tx);
        loop {
            match select3(reader.read(ctx), pubsub.receive(), writer.run(ctx)).await {
                Either3::First(Ok(Some(message))) => pubsub.publish(id, message),
                Either3::First(Ok(None)) => {
                    info!("[{id}] disconnected");
                    return Ok(());
                }
                Either3::First(Err(err)) => return Err(ClientError::Read { id, err }),
                Either3::Second(message) => writer.push(message),
                Either3::Third(written) => {
                    let Err(err) = written;
                    return Err(ClientError::Write { id, err });
                }
            }
        }
    }
}
