use anyhow::{Context, Result};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Clip {
    pub(crate) timestamp: u128,
    pub(crate) payload: Vec<u8>,
}

impl Clip {
    pub(crate) fn encode(self) -> Vec<u8> {
        let Self {
            timestamp,
            payload: mut data,
        } = self;
        let now: [u8; core::mem::size_of::<u128>()] = timestamp.to_be_bytes();
        data.extend_from_slice(&now);
        data
    }

    pub(crate) fn decode(mut bytes: Vec<u8>) -> Result<Self> {
        let split_idx = bytes
            .len()
            .checked_sub(core::mem::size_of::<u128>())
            .context("malformed message")?;

        let timestamp = u128::from_be_bytes(
            bytes
                .split_off(split_idx)
                .try_into()
                .map_err(|_| anyhow::anyhow!("bug: failed to parse message"))?,
        );

        Ok(Self {
            timestamp,
            payload: bytes,
        })
    }

    pub(crate) fn newer_than(&self, other: &Clip) -> bool {
        self.timestamp > other.timestamp
    }
}

impl std::fmt::Debug for Clip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = UNIX_EPOCH + Duration::from_nanos_u128(self.timestamp);
        let message = std::str::from_utf8(&self.payload);

        write!(f, "[at {time:?}] {message:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::Clip;

    #[test]
    fn test_encode() {
        let clip = Clip {
            timestamp: 12345678,
            payload: vec![1, 2, 3],
        };
        assert_eq!(
            clip.encode(),
            vec![1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 188, 97, 78]
        );
    }

    #[test]
    fn test_decode_ok() {
        assert_eq!(
            Clip::decode(vec![
                1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 188, 97, 78
            ])
            .unwrap(),
            Clip {
                timestamp: 12345678,
                payload: vec![1, 2, 3],
            }
        );
    }

    #[test]
    fn test_decode_err() {
        let decoded = Clip::decode(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 188, 97, 78]);
        assert!(decoded.is_err());
        assert_eq!(decoded.unwrap_err().to_string(), "malformed message")
    }
}
