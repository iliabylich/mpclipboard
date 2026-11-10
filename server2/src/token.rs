use crate::{
    constants::MAX_TOKEN_LENGTH,
    fixed_str::{FixedStr, FixedStrError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Token(FixedStr<MAX_TOKEN_LENGTH>);

impl Token {
    pub(crate) fn new(s: &str) -> Result<Self, FixedStrError> {
        FixedStr::new(s).map(Self)
    }
}
