pub fn has_token(value: &[u8], token: &str) -> bool {
    value
        .split(|byte| *byte == b',')
        .any(|item| item.trim_ascii().eq_ignore_ascii_case(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::has_token;

    #[test]
    fn test_single() {
        assert!(has_token(b"Upgrade", "upgrade"));
        assert!(!has_token(b"keep-alive", "upgrade"));
    }

    #[test]
    fn test_list() {
        assert!(has_token(b"keep-alive, Upgrade", "upgrade"));
        assert!(has_token(b"close,upgrade", "upgrade"));
    }

    #[test]
    fn test_whitespace() {
        assert!(has_token(b"  Upgrade\t", "upgrade"));
    }

    #[test]
    fn test_no_partial_match() {
        assert!(!has_token(b"upgrades", "upgrade"));
        assert!(!has_token(b"", "upgrade"));
    }
}
