use crate::ParseClipError;

/// Representation of the inner part of the Clip
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TextOrBinary {
    /// Text payload
    Text(String),

    /// Binary payload (e.g. file content)
    Binary(Vec<u8>),
}

const TEXT: u8 = 1;
const BINARY: u8 = 2;

impl TextOrBinary {
    pub(crate) fn encode(self) -> Vec<u8> {
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

    pub(crate) fn decode(mut bytes: Vec<u8>) -> Result<Self, ParseClipError> {
        let mark = bytes.pop().ok_or(ParseClipError)?;
        match mark {
            TEXT => {
                let string = String::from_utf8(bytes).map_err(|_| ParseClipError)?;
                Ok(Self::Text(string))
            }
            BINARY => Ok(Self::Binary(bytes)),
            _ => Err(ParseClipError),
        }
    }
}
