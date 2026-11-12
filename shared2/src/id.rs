use crate::{FixedStr, FixedStrError};

const MAX_ID_LENGTH: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ID(FixedStr<MAX_ID_LENGTH>);

impl ID {
    pub fn new(s: &str) -> Result<Self, FixedStrError> {
        FixedStr::new(s).map(Self)
    }
}

impl core::fmt::Display for ID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0.as_str())
    }
}
