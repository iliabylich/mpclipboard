use crate::Message;
use core::convert::Infallible;
use externally_driven_async_rt::{
    Context, State,
    io::{AsyncWrite, FrameWriter, FrameWriterError},
};

pub struct MessageWriter<W> {
    frames: FrameWriter<W, { Message::BYTESIZE }>,
    next: Option<Message>,
}

impl<W: AsyncWrite> MessageWriter<W> {
    pub const fn new(inner: W) -> Self {
        Self {
            frames: FrameWriter::new(inner),
            next: None,
        }
    }

    pub const fn push(&mut self, message: Message) {
        self.next = Some(message);
    }

    pub async fn run<
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &mut self,
        ctx: &Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
    ) -> Result<Infallible, FrameWriterError<W::Error>> {
        if let Some(message) = self.next {
            self.frames.write_all(ctx, &message.encode()).await?;
            self.next = None;
        }
        self.frames.flush(ctx).await?;
        core::future::pending().await
    }
}
