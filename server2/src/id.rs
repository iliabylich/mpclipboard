use crate::{
    constants::MAX_ID_LENGTH,
    fixed_str::{FixedStr, FixedStrError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ID(FixedStr<MAX_ID_LENGTH>);

impl ID {
    pub(crate) fn new(s: &str) -> Result<Self, FixedStrError> {
        FixedStr::new(s).map(Self)
    }
}

impl core::fmt::Display for ID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0.as_str())
    }
}
