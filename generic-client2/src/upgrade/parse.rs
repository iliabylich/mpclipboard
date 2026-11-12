use crate::{
    constants::{UPGRADE_RESPONSE_BUFFER_SIZE, UPGRADE_RESPONSE_MAX_HEADERS},
    upgrade::UpgradeError,
};
use mpclipboard_shared2::{UPGRADE_PROTOCOL, http::has_token};

pub(crate) fn parse(bytes: &[u8]) -> Result<bool, UpgradeError> {
    let mut headers = [httparse::EMPTY_HEADER; UPGRADE_RESPONSE_MAX_HEADERS];
    let mut res = httparse::Response::new(&mut headers);
    let httparse::Status::Complete(_) = res.parse(bytes)? else {
        if bytes.len() >= UPGRADE_RESPONSE_BUFFER_SIZE {
            return Err(UpgradeError::TooLarge);
        }
        return Ok(false);
    };

    match res.code {
        Some(101) => {}
        Some(401) => return Err(UpgradeError::Unauthorized),
        code => return Err(UpgradeError::UnexpectedStatus(code)),
    }

    let mut connection_upgrade = false;
    let mut upgrade = false;
    for header in res.headers.iter() {
        if header.name.eq_ignore_ascii_case("Connection") {
            connection_upgrade = has_token(header.value, "upgrade");
        } else if header.name.eq_ignore_ascii_case("Upgrade") {
            upgrade = has_token(header.value, UPGRADE_PROTOCOL);
        }
    }
    if !connection_upgrade {
        return Err(UpgradeError::NotUpgrade);
    }
    if !upgrade {
        return Err(UpgradeError::UnsupportedUpgrade);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{UpgradeError, parse};
    use crate::constants::UPGRADE_RESPONSE_BUFFER_SIZE;

    const RESPONSE: &str = "HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
\r\n";

    fn read_all(bytes: &[u8]) -> Result<Option<usize>, UpgradeError> {
        for len in 1..=bytes.len() {
            if parse(&bytes[..len])? {
                return Ok(Some(len));
            }
        }
        Ok(None)
    }

    #[test]
    fn test_no_leftover() {
        assert_eq!(read_all(RESPONSE.as_bytes()).unwrap(), Some(RESPONSE.len()));
    }

    #[test]
    fn test_leftover() {
        let bytes = [RESPONSE.as_bytes(), &[1; 3 * 256]].concat();
        assert_eq!(read_all(&bytes).unwrap(), Some(RESPONSE.len()));
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(
            read_all(&RESPONSE.as_bytes()[..RESPONSE.len() - 2]).unwrap(),
            None
        );
    }

    #[test]
    fn test_caddy_headers() {
        let bytes = RESPONSE.replace(
            "\r\n\r\n",
            "\r\nVia: 1.1 Caddy\r\nAlt-Svc: h3=\":8443\"; ma=2592000\r\n\r\n",
        );
        assert_eq!(read_all(bytes.as_bytes()).unwrap(), Some(bytes.len()));
    }

    #[test]
    fn test_unauthorized() {
        let bytes = "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";
        assert!(matches!(
            read_all(bytes.as_bytes()),
            Err(UpgradeError::Unauthorized)
        ));
    }

    #[test]
    fn test_unexpected_status() {
        let bytes = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";
        assert!(matches!(
            read_all(bytes.as_bytes()),
            Err(UpgradeError::UnexpectedStatus(Some(502)))
        ));
    }

    #[test]
    fn test_not_upgrade() {
        let bytes = RESPONSE.replace("Connection: Upgrade\r\n", "");
        assert!(matches!(
            read_all(bytes.as_bytes()),
            Err(UpgradeError::NotUpgrade)
        ));
    }

    #[test]
    fn test_unsupported_upgrade() {
        let bytes = RESPONSE.replace("Upgrade: mpclipboard-raw", "Upgrade: websocket");
        assert!(matches!(
            read_all(bytes.as_bytes()),
            Err(UpgradeError::UnsupportedUpgrade)
        ));
    }

    #[test]
    fn test_too_large() {
        let bytes = RESPONSE.replace(
            "\r\n\r\n",
            &format!(
                "\r\nX-Long: {}\r\n\r\n",
                "a".repeat(UPGRADE_RESPONSE_BUFFER_SIZE)
            ),
        );
        assert!(matches!(
            read_all(bytes.as_bytes()),
            Err(UpgradeError::TooLarge)
        ));
    }

    #[test]
    fn test_garbage() {
        assert!(matches!(
            read_all(b"boo\r\n\r\n"),
            Err(UpgradeError::InvalidHttpResponse(_))
        ));
    }
}
