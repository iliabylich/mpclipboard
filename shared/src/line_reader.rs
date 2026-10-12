use crate::{Buffer, PushResult, prelude::*};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LineReader<const N: usize> {
    LineWaitingForSlashR(Buffer<N>),
    LineWaitingForSlashN(Buffer<N>),
    SkipWaitingForSlashR,
    SkipWaitingForSlashN,
}

impl<const N: usize> LineReader<N> {
    pub(crate) const fn new() -> Self {
        Self::LineWaitingForSlashR(Buffer::empty())
    }

    pub(crate) fn push(&mut self, byte: u8) -> Result<Completion<Buffer<N>, ()>, LineReaderError> {
        match self {
            Self::LineWaitingForSlashR(buf) => match byte {
                b'\r' => {
                    *self = Self::LineWaitingForSlashN(*buf);
                    Ok(Pending(()))
                }

                b'\n' => Err(LineReaderError::BareLF),

                byte => {
                    if let PushResult::Full(_) = buf.push(byte) {
                        *self = Self::SkipWaitingForSlashR;
                    }
                    Ok(Pending(()))
                }
            },

            Self::LineWaitingForSlashN(buf) => match byte {
                b'\n' => {
                    let line = *buf;
                    *self = Self::new();
                    Ok(Done(line))
                }
                _ => Err(LineReaderError::BareCR),
            },

            Self::SkipWaitingForSlashR => match byte {
                b'\r' => {
                    *self = Self::SkipWaitingForSlashN;
                    Ok(Pending(()))
                }
                b'\n' => Err(LineReaderError::BareLF),
                _ => Ok(Pending(())),
            },

            Self::SkipWaitingForSlashN => match byte {
                b'\n' => {
                    *self = Self::new();
                    Ok(Pending(()))
                }
                _ => Err(LineReaderError::BareCR),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LineReaderError {
    #[error("bare CR in HTTP line")]
    BareCR,
    #[error("bare LF in HTTP line")]
    BareLF,
}

#[cfg(test)]
mod tests {
    use super::{LineReader, LineReaderError};
    use crate::prelude::*;
    use alloc::string::String;
    use alloc::{string::ToString, vec, vec::Vec};

    fn lines(input: &[u8]) -> Result<Vec<String>, LineReaderError> {
        let mut reader = LineReader::<5>::new();
        let mut out = vec![];
        for &byte in input {
            if let Done(line) = reader.push(byte)? {
                out.push(String::from_utf8_lossy(line.as_slice()).into_owned());
            }
        }
        Ok(out)
    }

    #[test]
    fn test_lines() {
        assert_eq!(
            lines(b"foo\r\nbar\r\n\r\n"),
            Ok(vec!["foo".to_string(), "bar".to_string(), String::new()])
        );
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(lines(b"foo\r\nbar\r"), Ok(vec!["foo".to_string()]));
    }

    #[test]
    fn test_max_len() {
        assert_eq!(lines(b"abcde\r\n"), Ok(vec!["abcde".to_string()]));
    }

    #[test]
    fn test_skip_long() {
        assert_eq!(lines(b"abcdef\r\nfoo\r\n"), Ok(vec!["foo".to_string()]));
    }

    #[test]
    fn test_bare_cr() {
        assert_eq!(lines(b"a\rb\r\n"), Err(LineReaderError::BareCR));
        assert_eq!(lines(b"a\r\r\n"), Err(LineReaderError::BareCR));
        assert_eq!(lines(b"abcdef\rb\r\n"), Err(LineReaderError::BareCR));
    }

    #[test]
    fn test_bare_lf() {
        assert_eq!(lines(b"a\nb\r\n"), Err(LineReaderError::BareLF));
        assert_eq!(lines(b"abcdef\nb\r\n"), Err(LineReaderError::BareLF));
    }
}
