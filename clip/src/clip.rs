use crate::ParseClipError;
use std::time::{SystemTime, UNIX_EPOCH};

/// A clip is a single message sent from a server to a client and vice versa.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Clip {
    /// A timestamp of when the payload was created, in nanoseconds
    pub timestamp: u128,

    /// A payload, must be a valid UTF-8 text
    pub text: String,
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos()
}

impl Clip {
    /// Constructs a Clip with a text content
    pub fn text(text: String) -> Self {
        Self {
            timestamp: now(),
            text,
        }
    }

    /// Converts self to a byte array, includes the timestamp and the
    /// information about when the Clip was created
    pub fn encode(self) -> Vec<u8> {
        let Self { timestamp, text } = self;

        let mut bytes = text.into_bytes();
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

        let text = String::from_utf8(bytes).map_err(|_| ParseClipError)?;

        Ok(Self { timestamp, text })
    }

    /// Returns true if self has a timestamp greater than the one in the given Clip
    pub fn newer_than(&self, other: &Clip) -> bool {
        self.timestamp > other.timestamp && self.text != other.text
    }

    /// Builds a dummy Clip with zero timestamp
    pub fn zero() -> Self {
        Self {
            timestamp: 0,
            text: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Clip, ParseClipError};

    #[test]
    fn test_encode() {
        let clip = Clip {
            timestamp: 87654321,
            text: String::from("abc"),
        };
        assert_eq!(
            clip.encode(),
            vec![
                97, 98, 99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 57, 127, 177
            ]
        );
    }

    #[test]
    fn test_decode_ok() {
        assert_eq!(
            Clip::decode(vec![
                97, 98, 99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 57, 127, 177
            ])
            .unwrap(),
            Clip {
                timestamp: 87654321,
                text: String::from("abc"),
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
