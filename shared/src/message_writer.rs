use crate::Message;
use externally_driven_async_rt::io::{AsyncWrite, FrameWriter, FrameWriterError};

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

    pub async fn run(&mut self) -> Result<!, FrameWriterError<W::Error>> {
        if let Some(message) = self.next {
            self.frames.write_all(&message.encode()).await?;
            self.next = None;
        }
        self.frames.flush().await?;
        core::future::pending().await
    }
}
