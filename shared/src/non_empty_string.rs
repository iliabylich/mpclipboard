use alloc::boxed::Box;

#[must_use]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyString(Box<str>);

impl NonEmptyString {
    pub fn new(s: impl Into<Box<str>>) -> Result<Self, NonEmptyStringError> {
        let s = s.into();
        if s.is_empty() {
            return Err(NonEmptyStringError::Empty);
        }
        Ok(Self(s))
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl core::fmt::Debug for NonEmptyString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl core::fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NonEmptyStringError {
    #[error("string is empty")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::{NonEmptyString, NonEmptyStringError};

    #[test]
    fn test_new() {
        assert_eq!(
            NonEmptyString::new("abc")
                .as_ref()
                .map(NonEmptyString::as_str),
            Ok("abc")
        );
        assert_eq!(
            NonEmptyString::new(String::from("abc"))
                .as_ref()
                .map(NonEmptyString::as_str),
            Ok("abc")
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(NonEmptyString::new(""), Err(NonEmptyStringError::Empty));
    }
}
