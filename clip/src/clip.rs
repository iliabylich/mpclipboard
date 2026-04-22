use crate::{ParseClipError, TextOrBinary};
use std::time::{SystemTime, UNIX_EPOCH};

/// A clip is a single message sent from a server to a client and vice versa.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Clip {
    /// A timestamp of when the payload was created, in nanoseconds
    pub timestamp: u128,

    /// A payload, either valid UTF-8 text or a binary blob
    pub text_or_binary: TextOrBinary,
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos()
}

impl Clip {
    /// Constructs a Clip with a binary content
    pub fn binary(bytes: Vec<u8>) -> Self {
        Self {
            timestamp: now(),
            text_or_binary: TextOrBinary::Binary(bytes),
        }
    }

    /// Constructs a Clip with a text content
    pub fn text(text: String) -> Self {
        Self {
            timestamp: now(),
            text_or_binary: TextOrBinary::Text(text),
        }
    }

    /// Converts self to a byte array, includes the timestamp and the
    /// information about when the Clip was created
    pub fn encode(self) -> Vec<u8> {
        let Self {
            timestamp,
            text_or_binary,
        } = self;

        let mut bytes = text_or_binary.encode();
        bytes.extend_from_slice(&timestamp.to_be_bytes());
        bytes
    }

    /// Decodes itself from a byte array
    pub fn decode(mut bytes: Vec<u8>) -> Result<Self, ParseClipError> {
        let split_idx = bytes
            .len()
            .checked_sub(size_of::<u128>())
            .ok_or(ParseClipError)?;

        let timestamp = u128::from_be_bytes(
            bytes
                .split_off(split_idx)
                .try_into()
                .map_err(|_| ParseClipError)?,
        );

        Ok(Self {
            timestamp,
            text_or_binary: TextOrBinary::decode(bytes)?,
        })
    }

    /// Returns true if self has a timestamp greater than the one in the given Clip
    pub fn newer_than(&self, other: &Clip) -> bool {
        self.timestamp > other.timestamp
    }

    /// Builds a dummy Clip with zero timestamp
    pub fn zero() -> Self {
        Self {
            timestamp: 0,
            text_or_binary: TextOrBinary::Binary(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Clip, ParseClipError, TextOrBinary};

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
        assert_eq!(decoded.unwrap_err(), ParseClipError)
    }
}
