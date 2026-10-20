use anyhow::{Context as _, Result, anyhow, ensure};
use bytes::{Buf, BytesMut};
use core::num::NonZeroUsize;

#[must_use]
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ChunkedDecoder {
    buf: BytesMut,
    state: State,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum State {
    #[default]
    Size,
    Data(NonZeroUsize),
    DataCRLF,
}

impl ChunkedDecoder {
    const MAX_SIZE_LINE_LENGTH: usize = 8192;

    pub(crate) fn decode(&mut self, input: &[u8]) -> Result<BytesMut> {
        self.buf.extend_from_slice(input);
        let mut out = BytesMut::new();

        loop {
            match self.state {
                State::Size => {
                    let status = httparse::parse_chunk_size(&self.buf)
                        .map_err(|_| anyhow!("invalid chunk size line"))?;
                    let httparse::Status::Complete((consumed, size)) = status else {
                        ensure!(
                            self.buf.len() <= Self::MAX_SIZE_LINE_LENGTH,
                            "chunk size line exceeds {} bytes",
                            Self::MAX_SIZE_LINE_LENGTH
                        );
                        break;
                    };
                    self.buf.advance(consumed);
                    let size =
                        usize::try_from(size).context("chunk size doesn't fit into usize")?;
                    let size = NonZeroUsize::new(size).context("request body has ended")?;
                    self.state = State::Data(size);
                }
                State::Data(remaining) => {
                    if self.buf.is_empty() {
                        break;
                    }
                    let len = remaining.get().min(self.buf.len());
                    out.extend_from_slice(&self.buf.split_to(len));
                    self.state = match NonZeroUsize::new(remaining.get().saturating_sub(len)) {
                        Some(remaining) => State::Data(remaining),
                        None => State::DataCRLF,
                    };
                }
                State::DataCRLF => {
                    if self.buf.len() < 2 {
                        break;
                    }
                    ensure!(
                        self.buf.split_to(2).as_ref() == b"\r\n",
                        "chunk data is not followed by CRLF"
                    );
                    self.state = State::Size;
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChunkedDecoder, State};

    fn decode(decoder: &mut ChunkedDecoder, input: &[u8]) -> Result<Vec<u8>, String> {
        decoder
            .decode(input)
            .map(|output| output.to_vec())
            .map_err(|err| err.to_string())
    }

    #[test]
    fn test_single_chunk() {
        let mut decoder = ChunkedDecoder::default();
        assert_eq!(
            decode(&mut decoder, b"5\r\nhello\r\n"),
            Ok(b"hello".to_vec())
        );
        assert_eq!(decoder, ChunkedDecoder::default());
    }

    #[test]
    fn test_many_chunks() {
        let mut decoder = ChunkedDecoder::default();
        assert_eq!(
            decode(&mut decoder, b"3\r\nabc\r\nA\r\n0123456789\r\n"),
            Ok(b"abc0123456789".to_vec())
        );
    }

    #[test]
    fn test_byte_by_byte() {
        let mut decoder = ChunkedDecoder::default();
        let mut out = vec![];
        for byte in b"1a\r\nabcdefghijklmnopqrstuvwxyz\r\n" {
            out.extend(decode(&mut decoder, &[*byte]).unwrap());
        }
        assert_eq!(out, b"abcdefghijklmnopqrstuvwxyz");
        assert_eq!(decoder.state, State::Size);
    }

    #[test]
    fn test_extension() {
        let mut decoder = ChunkedDecoder::default();
        assert_eq!(
            decode(&mut decoder, b"3;name=value\r\nabc\r\n"),
            Ok(b"abc".to_vec())
        );
    }

    #[test]
    fn test_end_of_body() {
        let mut decoder = ChunkedDecoder::default();
        assert_eq!(
            decode(&mut decoder, b"3\r\nabc\r\n0\r\n\r\n"),
            Err("request body has ended".to_string())
        );
    }

    #[test]
    fn test_size_line_too_long() {
        let mut decoder = ChunkedDecoder::default();
        assert_eq!(decode(&mut decoder, b"3;"), Ok(vec![]));
        assert_eq!(
            decode(&mut decoder, &[b'a'; ChunkedDecoder::MAX_SIZE_LINE_LENGTH]),
            Err("chunk size line exceeds 8192 bytes".to_string())
        );
    }

    #[test]
    fn test_errors() {
        assert_eq!(
            decode(&mut ChunkedDecoder::default(), b"x\r\n"),
            Err("invalid chunk size line".to_string())
        );
        assert_eq!(
            decode(&mut ChunkedDecoder::default(), b"11111111111111111\r\n"),
            Err("invalid chunk size line".to_string())
        );
        assert_eq!(
            decode(&mut ChunkedDecoder::default(), b"3\rabc"),
            Err("invalid chunk size line".to_string())
        );
        assert_eq!(
            decode(&mut ChunkedDecoder::default(), b"3\r\nabcde"),
            Err("chunk data is not followed by CRLF".to_string())
        );
    }
}
