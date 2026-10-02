use crate::{Buffer, prelude::*};

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
        Self::LineWaitingForSlashR(Buffer::new())
    }

    pub(crate) fn push(&mut self, byte: u8) -> Completion<Buffer<N>, LineReaderError, ()> {
        match self {
            Self::LineWaitingForSlashR(buf) => match byte {
                b'\r' => {
                    *self = Self::LineWaitingForSlashN(*buf);
                    Pending(())
                }

                b'\n' => Failed(LineReaderError::BareLF),

                byte => {
                    if !buf.push(byte) {
                        *self = Self::SkipWaitingForSlashR;
                    }
                    Pending(())
                }
            },

            Self::LineWaitingForSlashN(buf) => match byte {
                b'\n' => {
                    let line = *buf;
                    *self = Self::new();
                    Done(line)
                }
                _ => Failed(LineReaderError::BareCR),
            },

            Self::SkipWaitingForSlashR => match byte {
                b'\r' => {
                    *self = Self::SkipWaitingForSlashN;
                    Pending(())
                }
                b'\n' => Failed(LineReaderError::BareLF),
                _ => Pending(()),
            },

            Self::SkipWaitingForSlashN => match byte {
                b'\n' => {
                    *self = Self::new();
                    Pending(())
                }
                _ => Failed(LineReaderError::BareCR),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineReaderError {
    BareCR,
    BareLF,
}

impl core::fmt::Display for LineReaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BareCR => write!(f, "bare CR in HTTP line"),
            Self::BareLF => write!(f, "bare LF in HTTP line"),
        }
    }
}

impl core::error::Error for LineReaderError {}

#[cfg(test)]
mod tests {
    use super::{LineReader, LineReaderError};
    use crate::prelude::*;
    use alloc::{string::ToString, vec, vec::Vec};
    use alloc::string::String;

    fn lines(input: &[u8]) -> Result<Vec<String>, LineReaderError> {
        let mut reader = LineReader::<5>::new();
        let mut out = vec![];
        for &byte in input {
            match reader.push(byte) {
                Done(line) => {
                    out.push(String::from_utf8_lossy(line.as_slice()).into_owned());
                }
                Failed(err) => return Err(err),
                Pending(()) => {}
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
