/// Parse error
#[derive(Debug, PartialEq, Eq)]
pub struct ParseClipError;

impl std::fmt::Display for ParseClipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseClipError {}
