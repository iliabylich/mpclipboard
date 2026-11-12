use crate::{Message, MessageDecodeError};
use externally_driven_async_rt::{
    Context, State,
    io::{AsyncRead, FrameReader, FrameReaderError},
};

pub struct MessageReader<R> {
    frames: FrameReader<R, { Message::BYTESIZE }>,
}

impl<R: AsyncRead> MessageReader<R> {
    pub const fn new(inner: R) -> Self {
        Self {
            frames: FrameReader::new(inner),
        }
    }

    pub async fn read<
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &mut self,
        ctx: &Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
    ) -> Result<Option<Message>, MessageReaderError<R::Error>> {
        let Some(frame) = self.frames.read_exact(ctx).await? else {
            return Ok(None);
        };
        let message = Message::decode(&frame)?;
        Ok(Some(message))
    }
}

impl<R> MessageReader<R> {
    pub const fn get_mut(&mut self) -> &mut R {
        self.frames.get_mut()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageReaderError<E> {
    Frame(FrameReaderError<E>),
    Decode(MessageDecodeError),
}

impl<E> From<FrameReaderError<E>> for MessageReaderError<E> {
    fn from(err: FrameReaderError<E>) -> Self {
        Self::Frame(err)
    }
}

impl<E> From<MessageDecodeError> for MessageReaderError<E> {
    fn from(err: MessageDecodeError) -> Self {
        Self::Decode(err)
    }
}

impl<E: core::fmt::Display> core::fmt::Display for MessageReaderError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Frame(err) => write!(f, "failed to read a frame: {err}"),
            Self::Decode(err) => write!(f, "failed to decode a message: {err}"),
        }
    }
}

impl<E: core::error::Error> core::error::Error for MessageReaderError<E> {}
