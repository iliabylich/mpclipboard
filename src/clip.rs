use anyhow::{Context, Result, bail};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum TextOrBinary {
    Text(String),
    Binary(Vec<u8>),
}

const TEXT: u8 = 1;
const BINARY: u8 = 2;
const ERROR: &str = "malformed message";

impl TextOrBinary {
    fn encode(self) -> Vec<u8> {
        match self {
            TextOrBinary::Text(text) => {
                let mut bytes = text.into_bytes();
                bytes.push(TEXT);
                bytes
            }
            TextOrBinary::Binary(mut bytes) => {
                bytes.push(BINARY);
                bytes
            }
        }
    }

    fn decode(mut bytes: Vec<u8>) -> Result<Self> {
        let mark = bytes.pop().context(ERROR)?;
        match mark {
            TEXT => {
                let string = String::from_utf8(bytes).context(ERROR)?;
                Ok(Self::Text(string))
            }
            BINARY => Ok(Self::Binary(bytes)),
            _ => bail!(ERROR),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Clip {
    pub(crate) timestamp: u128,
    text_or_binary: TextOrBinary,
}

impl Clip {
    pub(crate) fn encode(self) -> Vec<u8> {
        let Self {
            timestamp,
            text_or_binary,
        } = self;

        let mut bytes = text_or_binary.encode();
        bytes.extend_from_slice(&timestamp.to_be_bytes());
        bytes
    }

    pub(crate) fn decode(mut bytes: Vec<u8>) -> Result<Self> {
        let split_idx = bytes
            .len()
            .checked_sub(size_of::<u128>())
            .context("malformed message")?;

        let timestamp = u128::from_be_bytes(
            bytes
                .split_off(split_idx)
                .try_into()
                .map_err(|_| anyhow::anyhow!("bug: failed to parse message"))?,
        );

        Ok(Self {
            timestamp,
            text_or_binary: TextOrBinary::decode(bytes)?,
        })
    }

    pub(crate) fn newer_than(&self, other: &Clip) -> bool {
        self.timestamp > other.timestamp
    }

    pub(crate) fn zero() -> Self {
        Self {
            timestamp: 0,
            text_or_binary: TextOrBinary::Binary(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Clip, TextOrBinary};

    #[test]
    fn test_encode() {
        let clip = Clip {
            timestamp: 12345678,
            text_or_binary: TextOrBinary::Binary(vec![1, 2, 3]),
        };
        assert_eq!(
            clip.encode(),
            vec![
                1, 2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 188, 97, 78
            ]
        );

        let clip = Clip {
            timestamp: 87654321,
            text_or_binary: TextOrBinary::Text(String::from("abc")),
        };
        assert_eq!(
            clip.encode(),
            vec![
                97, 98, 99, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 57, 127, 177
            ]
        );
    }

    #[test]
    fn test_decode_ok() {
        assert_eq!(
            Clip::decode(vec![
                1, 2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 188, 97, 78
            ])
            .unwrap(),
            Clip {
                timestamp: 12345678,
                text_or_binary: TextOrBinary::Binary(vec![1, 2, 3]),
            }
        );

        assert_eq!(
            Clip::decode(vec![
                97, 98, 99, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 57, 127, 177
            ])
            .unwrap(),
            Clip {
                timestamp: 87654321,
                text_or_binary: TextOrBinary::Text(String::from("abc")),
            }
        );
    }

    #[test]
    fn test_decode_err() {
        let decoded = Clip::decode(vec![42]);
        assert!(decoded.is_err());
        assert_eq!(decoded.unwrap_err().to_string(), "malformed message")
    }
}
