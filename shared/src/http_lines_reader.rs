use crate::http_lines_buffer::{
    HttpLinesBuffer, HttpLinesBufferOverflowError, HttpLinesBufferUnderflowError,
};
use core::{num::NonZeroUsize, str::Utf8Error};

#[derive(Debug, Clone, Copy)]
pub struct HttpLinesReader<P, const N: usize>
where
    P: HttpLinesParser,
{
    buf: HttpLinesBuffer<N>,
    seen_end: bool,
    parser: P,
}

impl<P, const N: usize> HttpLinesReader<P, N>
where
    P: HttpLinesParser,
{
    pub(crate) const fn new(parser: P) -> Self {
        Self {
            buf: HttpLinesBuffer::new(),
            seen_end: false,
            parser,
        }
    }

    pub(crate) fn received(
        &mut self,
        data: &[u8],
    ) -> Result<(usize, Option<P::Output>), HttpLinesReaderError<P::Error>> {
        let received = {
            let remainder = self.buf.remainder();
            let len = remainder.len().min(data.len());
            remainder
                .get_mut(..len)
                .unwrap_or_else(|| unreachable!("write range exceeds HTTP buffer"))
                .copy_from_slice(
                    data.get(..len)
                        .unwrap_or_else(|| unreachable!("write range exceeds input")),
                );
            if let Some(len) = NonZeroUsize::new(len) {
                self.buf
                    .received(len)
                    .map_err(HttpLinesReaderError::BufferOverflow)?;
            }

            len
        };

        while let Some(line) = self.buf.next_line() {
            let line = core::str::from_utf8(line).map_err(HttpLinesReaderError::InvalidUtf8)?;

            if line == "\r\n" {
                self.seen_end = true;
            } else {
                self.parser
                    .line_received(line)
                    .map_err(HttpLinesReaderError::Parser)?;
            }
            self.buf
                .consumed(line.len())
                .map_err(HttpLinesReaderError::BufferUnderflow)?;

            if self.seen_end {
                let output = self
                    .parser
                    .try_finish()
                    .ok_or(HttpLinesReaderError::IncompleteUpgradeRequest)?;
                let consumed = received
                    .checked_sub(self.buf.len())
                    .unwrap_or_else(|| unreachable!("HTTP leftover exceeds received input"));
                return Ok((consumed, Some(output)));
            }
        }

        if received != data.len() || self.buf.remainder().is_empty() {
            return Err(HttpLinesReaderError::UpgradeRequestExceedsBufferSize);
        }

        Ok((received, None))
    }
}

#[derive(Debug)]
pub enum HttpLinesReaderError<E> {
    BufferOverflow(HttpLinesBufferOverflowError),
    BufferUnderflow(HttpLinesBufferUnderflowError),
    InvalidUtf8(Utf8Error),
    Parser(E),
    IncompleteUpgradeRequest,
    UpgradeRequestExceedsBufferSize,
}

impl<E: core::fmt::Display> core::fmt::Display for HttpLinesReaderError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferOverflow(error) => error.fmt(f),
            Self::BufferUnderflow(error) => error.fmt(f),
            Self::InvalidUtf8(error) => write!(f, "non-utf8 upgrade request line: {error}"),
            Self::Parser(error) => error.fmt(f),
            Self::IncompleteUpgradeRequest => {
                f.write_str("incomplete HTTP upgrade upgrade request")
            }
            Self::UpgradeRequestExceedsBufferSize => {
                f.write_str("HTTP upgrade request exceeds buffer size")
            }
        }
    }
}

impl<E: core::error::Error + 'static> core::error::Error for HttpLinesReaderError<E> {}

pub trait HttpLinesParser {
    type Output;
    type Error;

    fn new() -> Self;
    fn line_received(&mut self, line: &str) -> Result<(), Self::Error>;
    fn try_finish(&self) -> Option<Self::Output>;
}
