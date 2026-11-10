use crate::handshake::Token;
use core::net::SocketAddr;

pub(crate) struct Config {
    pub(crate) bind_address: SocketAddr,
    pub(crate) token: Token,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self, ()> {
        let bind_address = crate::env::var(c"MPCLIPBOARD_BIND_ADDRESS")?;
        let token = crate::env::var(c"MPCLIPBOARD_TOKEN")?;
        Self::parse(bind_address, token)
    }

    fn parse(bind_address: Option<&str>, token: Option<&str>) -> Result<Self, ()> {
        let Some(bind_address) = bind_address else {
            log::error!("MPCLIPBOARD_BIND_ADDRESS is missing");
            return Err(());
        };
        let bind_address = bind_address
            .parse::<SocketAddr>()
            .inspect_err(|err| {
                log::error!(
                    "MPCLIPBOARD_BIND_ADDRESS=\"{bind_address}\" must be an ip:port address: {err}"
                );
            })
            .map_err(|_| ())?;

        let Some(token) = token else {
            log::error!("MPCLIPBOARD_TOKEN is missing");
            return Err(());
        };
        let token = Token::new(token)
            .inspect_err(|err| log::error!("MPCLIPBOARD_TOKEN is malformed: {err}"))
            .map_err(|_| ())?;

        Ok(Self {
            bind_address,
            token,
        })
    }
}

impl core::fmt::Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Config {{ bind_address: {}, token: \"******\" }}",
            self.bind_address
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    fn bind_address(value: &str) -> Result<std::string::String, ()> {
        let config = Config::parse(Some(value), Some("sekret"))?;
        Ok(std::format!("{}", config.bind_address))
    }

    #[test]
    fn test_ipv4() {
        assert_eq!(bind_address("127.0.0.1:3000"), Ok("127.0.0.1:3000".into()));
        assert_eq!(bind_address("0.0.0.0:3000"), Ok("0.0.0.0:3000".into()));
    }

    #[test]
    fn test_ipv6() {
        assert_eq!(bind_address("[::1]:3000"), Ok("[::1]:3000".into()));
        assert_eq!(bind_address("[::]:3000"), Ok("[::]:3000".into()));
    }

    #[test]
    fn test_rejected_addresses() {
        assert_eq!(bind_address("localhost:3000"), Err(()));
        assert_eq!(bind_address("http://127.0.0.1:3000"), Err(()));
        assert_eq!(bind_address("127.0.0.1"), Err(()));
        assert_eq!(bind_address(""), Err(()));
    }

    #[test]
    fn test_missing() {
        assert!(Config::parse(None, Some("sekret")).is_err());
        assert!(Config::parse(Some("127.0.0.1:3000"), None).is_err());
    }

    #[test]
    fn test_token() {
        let config = Config::parse(Some("127.0.0.1:3000"), Some("sekret")).unwrap();
        assert_eq!(config.token.as_str(), "sekret");
        assert!(Config::parse(Some("127.0.0.1:3000"), Some("")).is_err());
    }
}
