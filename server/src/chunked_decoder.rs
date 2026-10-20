use core::num::NonZeroU64;
use mpclipboard_shared::{Buffer, PushResult};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChunkedDecoder {
    SizeStart,
    Size(u64),
    Extension(u64),
    SizeLF(u64),
    Data(NonZeroU64),
    DataCR,
    DataLF,
}

impl ChunkedDecoder {
    pub(crate) const fn new() -> Self {
        Self::SizeStart
    }

    pub(crate) fn decode<const N: usize>(
        &mut self,
        input: Buffer<N>,
    ) -> Result<Buffer<N>, ChunkedDecoderError> {
        let mut out = Buffer::<N>::empty();
        for &byte in input.as_slice() {
            if let Some(byte) = self.push(byte)?
                && let PushResult::Full(_) = out.push(byte)
            {
                unreachable!("decoded data is never longer than encoded data");
            }
        }
        Ok(out)
    }

    fn push(&mut self, byte: u8) -> Result<Option<u8>, ChunkedDecoderError> {
        match *self {
            Self::SizeStart => {
                let digit = hex_digit(byte).ok_or(ChunkedDecoderError::InvalidSize(byte))?;
                *self = Self::Size(digit);
            }
            Self::Size(size) => match byte {
                b';' => *self = Self::Extension(size),
                b'\r' => *self = Self::SizeLF(size),
                _ => {
                    let digit = hex_digit(byte).ok_or(ChunkedDecoderError::InvalidSize(byte))?;
                    let size = size
                        .checked_mul(16)
                        .and_then(|size| size.checked_add(digit))
                        .ok_or(ChunkedDecoderError::SizeOverflow)?;
                    *self = Self::Size(size);
                }
            },
            Self::Extension(size) => {
                if byte == b'\r' {
                    *self = Self::SizeLF(size);
                }
            }
            Self::SizeLF(size) => {
                if byte != b'\n' {
                    return Err(ChunkedDecoderError::MissingLF);
                }
                let size = NonZeroU64::new(size).ok_or(ChunkedDecoderError::EndOfBody)?;
                *self = Self::Data(size);
            }
            Self::Data(remaining) => {
                *self = remaining
                    .get()
                    .checked_sub(1)
                    .and_then(NonZeroU64::new)
                    .map_or(Self::DataCR, Self::Data);
                return Ok(Some(byte));
            }
            Self::DataCR => {
                if byte != b'\r' {
                    return Err(ChunkedDecoderError::MissingCRLF);
                }
                *self = Self::DataLF;
            }
            Self::DataLF => {
                if byte != b'\n' {
                    return Err(ChunkedDecoderError::MissingCRLF);
                }
                *self = Self::SizeStart;
            }
        }
        Ok(None)
    }
}

fn hex_digit(byte: u8) -> Option<u64> {
    char::from(byte).to_digit(16).map(u64::from)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChunkedDecoderError {
    InvalidSize(u8),
    SizeOverflow,
    MissingLF,
    MissingCRLF,
    EndOfBody,
}

impl core::fmt::Display for ChunkedDecoderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSize(byte) => write!(f, "invalid byte {byte:#04x} in chunk size"),
            Self::SizeOverflow => write!(f, "chunk size overflows u64"),
            Self::MissingLF => write!(f, "chunk size is not followed by LF"),
            Self::MissingCRLF => write!(f, "chunk data is not followed by CRLF"),
            Self::EndOfBody => write!(f, "request body has ended"),
        }
    }
}

impl core::error::Error for ChunkedDecoderError {}

#[cfg(test)]
mod tests {
    use super::{ChunkedDecoder, ChunkedDecoderError};
    use mpclipboard_shared::Buffer;

    fn decode(decoder: &mut ChunkedDecoder, input: &[u8]) -> Result<Vec<u8>, ChunkedDecoderError> {
        let input = Buffer::<256>::from_slice(input).unwrap();
        let output = decoder.decode(input)?;
        Ok(output.as_slice().to_vec())
    }

    #[test]
    fn test_single_chunk() {
        let mut decoder = ChunkedDecoder::new();
        assert_eq!(
            decode(&mut decoder, b"5\r\nhello\r\n"),
            Ok(b"hello".to_vec())
        );
        assert_eq!(decoder, ChunkedDecoder::SizeStart);
    }

    #[test]
    fn test_many_chunks() {
        let mut decoder = ChunkedDecoder::new();
        assert_eq!(
            decode(&mut decoder, b"3\r\nabc\r\nA\r\n0123456789\r\n"),
            Ok(b"abc0123456789".to_vec())
        );
    }

    #[test]
    fn test_byte_by_byte() {
        let mut decoder = ChunkedDecoder::new();
        let mut out = vec![];
        for byte in b"1a\r\nabcdefghijklmnopqrstuvwxyz\r\n" {
            out.extend(decode(&mut decoder, &[*byte]).unwrap());
        }
        assert_eq!(out, b"abcdefghijklmnopqrstuvwxyz");
    }

    #[test]
    fn test_extension() {
        let mut decoder = ChunkedDecoder::new();
        assert_eq!(
            decode(&mut decoder, b"3;name=value\r\nabc\r\n"),
            Ok(b"abc".to_vec())
        );
    }

    #[test]
    fn test_end_of_body() {
        let mut decoder = ChunkedDecoder::new();
        assert_eq!(
            decode(&mut decoder, b"3\r\nabc\r\n0\r\n\r\n"),
            Err(ChunkedDecoderError::EndOfBody)
        );
    }

    #[test]
    fn test_errors() {
        assert_eq!(
            decode(&mut ChunkedDecoder::new(), b"x\r\n"),
            Err(ChunkedDecoderError::InvalidSize(b'x'))
        );
        assert_eq!(
            decode(&mut ChunkedDecoder::new(), b"11111111111111111\r\n"),
            Err(ChunkedDecoderError::SizeOverflow)
        );
        assert_eq!(
            decode(&mut ChunkedDecoder::new(), b"3\rabc"),
            Err(ChunkedDecoderError::MissingLF)
        );
        assert_eq!(
            decode(&mut ChunkedDecoder::new(), b"3\r\nabcd"),
            Err(ChunkedDecoderError::MissingCRLF)
        );
    }
}
