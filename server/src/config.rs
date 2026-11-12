use crate::errors::ServerError;
use core::net::SocketAddr;
use mpclipboard_shared::Token;

pub(crate) fn from_env() -> Result<(SocketAddr, &'static str, Token), ServerError> {
    let (bind_address, raw_bind_address) =
        crate::env::var(c"MPCLIPBOARD_BIND_ADDRESS").and_then(parse_address)?;
    let token = crate::env::var(c"MPCLIPBOARD_TOKEN").and_then(parse_token)?;
    Ok((bind_address, raw_bind_address, token))
}

fn parse_address(s: Option<&str>) -> Result<(SocketAddr, &str), ServerError> {
    let s = s.ok_or(ServerError::MissingBindAddress)?;
    let address = s.parse::<SocketAddr>()?;
    Ok((address, s))
}

fn parse_token(s: Option<&str>) -> Result<Token, ServerError> {
    let s = s.ok_or(ServerError::MissingToken)?;
    Token::new(s).map_err(ServerError::InvalidToken)
}

#[cfg(test)]
mod tests {
    use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    use crate::errors::ServerError;
    use mpclipboard_shared::Token;

    use super::{parse_address, parse_token};

    fn ipv4(a: u8, b: u8, c: u8, d: u8, port: u16) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(a, b, c, d), port))
    }

    fn ipv6(
        a: u16,
        b: u16,
        c: u16,
        d: u16,
        e: u16,
        f: u16,
        g: u16,
        h: u16,
        port: u16,
        flowinfo: u32,
        scope_id: u32,
    ) -> SocketAddr {
        SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(a, b, c, d, e, f, g, h),
            port,
            flowinfo,
            scope_id,
        ))
    }

    #[test]
    fn test_ipv4() {
        assert_eq!(
            parse_address(Some("127.0.0.1:3000")),
            Ok((ipv4(127, 0, 0, 1, 3000), "127.0.0.1:3000"))
        );
        assert_eq!(
            parse_address(Some("0.0.0.0:80")),
            Ok((ipv4(0, 0, 0, 0, 80), "0.0.0.0:80"))
        );
    }

    #[test]
    fn test_ipv6() {
        assert_eq!(
            parse_address(Some("[::1]:3000")),
            Ok((ipv6(0, 0, 0, 0, 0, 0, 0, 1, 3000, 0, 0), "[::1]:3000"))
        );

        assert_eq!(
            parse_address(Some("[::]:3000")),
            Ok((ipv6(0, 0, 0, 0, 0, 0, 0, 0, 3000, 0, 0), "[::]:3000"))
        );
    }

    #[test]
    fn test_rejected_addresses() {
        assert!(matches!(
            parse_address(Some("localhost:3000")),
            Err(ServerError::InvalidBindAddress(_))
        ));
        assert!(matches!(
            parse_address(Some("http://127.0.0.1:3000")),
            Err(ServerError::InvalidBindAddress(_))
        ));
        assert!(matches!(
            parse_address(Some("127.0.0.1")),
            Err(ServerError::InvalidBindAddress(_))
        ));
        assert!(matches!(
            parse_address(Some("")),
            Err(ServerError::InvalidBindAddress(_))
        ));
    }

    #[test]
    fn test_missing() {
        assert_eq!(parse_address(None), Err(ServerError::MissingBindAddress));
        assert_eq!(parse_token(None), Err(ServerError::MissingToken));
    }

    #[test]
    fn test_token() {
        assert_eq!(
            parse_token(Some("sekret")).unwrap(),
            Token::new("sekret").unwrap()
        );
    }
}
