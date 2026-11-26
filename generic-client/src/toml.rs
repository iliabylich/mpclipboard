#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Toml<'a> {
    pub(crate) url: Option<&'a str>,
    pub(crate) token: Option<&'a str>,
    pub(crate) id: Option<&'a str>,
}

impl<'a> Toml<'a> {
    pub(crate) fn parse(text: &'a str) -> Result<Self, TomlError> {
        let mut toml = Self::default();
        for (lineno, line) in (1..).zip(text.lines()) {
            if line.is_empty() {
                continue;
            }
            let error = TomlError { lineno };

            let (key, value) = line.split_once('=').ok_or(TomlError { lineno })?;
            let key = key.trim();
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .ok_or(TomlError { lineno })?
                .strip_suffix('"')
                .ok_or(TomlError { lineno })?;

            if value.contains(['"', '\\']) {
                return Err(error);
            }

            let slot = match key {
                "url" => &mut toml.url,
                "token" => &mut toml.token,
                "id" => &mut toml.id,
                _ => return Err(error),
            };
            *slot = Some(value);
        }
        Ok(toml)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("malformed TOML at line {lineno}")]
pub struct TomlError {
    pub lineno: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;

    #[test]
    fn test_valid() {
        let text = "url = \"wss://example.com\"\n\ntoken = \"secret\"\nid = \"\"\n";
        assert_eq!(
            Toml::parse(text),
            Ok(Toml {
                url: Some("wss://example.com"),
                token: Some("secret"),
                id: Some(""),
            })
        );
    }

    #[test]
    fn test_invalid() {
        for line in [
            "foo = \"x\"",
            "id = 42",
            "id = 'x'",
            "id = \"x",
            "id = \"a\\nb\"",
            "id = \"a\"b\"",
            "id = \"x\" # comment",
            "[table]",
        ] {
            let text = format!("url = \"x\"\n{line}");
            assert_eq!(Toml::parse(&text), Err(TomlError { lineno: 2 }), "{line}");
        }
    }
}
