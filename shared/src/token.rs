use crate::{FixedStr, FixedStrError};

const MAX_TOKEN_LENGTH: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Token(FixedStr<MAX_TOKEN_LENGTH>);

impl Token {
    pub fn new(s: &str) -> Result<Self, FixedStrError> {
        FixedStr::new(s).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl core::fmt::Debug for Token {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("******")
    }
}
