use crate::{Buffer, prelude::*};
use anyhow::anyhow;

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

    pub(crate) fn push(&mut self, byte: u8) -> Completion<Buffer<N>, anyhow::Error, ()> {
        match self {
            Self::LineWaitingForSlashR(buf) => match byte {
                b'\r' => {
                    *self = Self::LineWaitingForSlashN(*buf);
                    Pending(())
                }

                b'\n' => Failed(anyhow!("bare LF in HTTP line")),

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
                _ => Failed(anyhow!("bare CR in HTTP line")),
            },

            Self::SkipWaitingForSlashR => match byte {
                b'\r' => {
                    *self = Self::SkipWaitingForSlashN;
                    Pending(())
                }
                b'\n' => Failed(anyhow!("bare LF in HTTP line")),
                _ => Pending(()),
            },

            Self::SkipWaitingForSlashN => match byte {
                b'\n' => {
                    *self = Self::new();
                    Pending(())
                }
                _ => Failed(anyhow!("bare CR in HTTP line")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LineReader;
    use crate::prelude::*;
    use anyhow::Result;

    fn lines(input: &[u8]) -> Result<Vec<String>> {
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
    fn test_lines() -> Result<()> {
        assert_eq!(
            lines(b"foo\r\nbar\r\n\r\n")?,
            vec!["foo".to_string(), "bar".to_string(), String::new()]
        );
        Ok(())
    }

    #[test]
    fn test_incomplete() -> Result<()> {
        assert_eq!(lines(b"foo\r\nbar\r")?, vec!["foo".to_string()]);
        Ok(())
    }

    #[test]
    fn test_max_len() -> Result<()> {
        assert_eq!(lines(b"abcde\r\n")?, vec!["abcde".to_string()]);
        Ok(())
    }

    #[test]
    fn test_skip_long() -> Result<()> {
        assert_eq!(lines(b"abcdef\r\nfoo\r\n")?, vec!["foo".to_string()]);
        Ok(())
    }

    #[test]
    fn test_bare_cr() {
        assert_eq!(
            lines(b"a\rb\r\n").map_err(|err| err.to_string()),
            Err("bare CR in HTTP line".to_string())
        );

        assert_eq!(
            lines(b"a\r\r\n").map_err(|err| err.to_string()),
            Err("bare CR in HTTP line".to_string())
        );

        assert_eq!(
            lines(b"abcdef\rb\r\n").map_err(|err| err.to_string()),
            Err("bare CR in HTTP line".to_string())
        );
    }

    #[test]
    fn test_bare_lf() {
        assert_eq!(
            lines(b"a\nb\r\n").map_err(|err| err.to_string()),
            Err("bare LF in HTTP line".to_string())
        );

        assert_eq!(
            lines(b"abcdef\nb\r\n").map_err(|err| err.to_string()),
            Err("bare LF in HTTP line".to_string())
        );
    }
}
