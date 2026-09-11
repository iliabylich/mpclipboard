use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HostPort, ID, ID_PREFIX, MAX_HOST_PORT_LENGTH,
    MAX_ID_LENGTH, MAX_TOKEN_LENGTH, MIN_PADDING_LENGTH, PADDING_PREFIX, START_LINE, TOKEN_PREFIX,
    Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub host: HostPort,
    pub token: Token,
    pub id: ID,
}

const _: () = assert!(HandshakeRequest::BYTESIZE == 555);

const BASE_HANDSHAKE_LENGTH: usize = START_LINE.len() + 2 // start line
    + HOST_PREFIX.len() + 2 // Host: ...
    + TOKEN_PREFIX.len() + 2 // Token: ...
    + ID_PREFIX.len() + 2 // ID: ...
    + CONNECTION_UPGRADE_HEADER.len() + 2 // Connection: Upgrade
    + UPGRADE_MPCLIPBOARD_RAW_HEADER.len() + 2 // Upgrade: mpclipboard-raw
    + PADDING_PREFIX.len() + 2 //
    + 2; // headers end marker

impl HandshakeRequest {
    pub const BYTESIZE: usize = BASE_HANDSHAKE_LENGTH
        + MAX_HOST_PORT_LENGTH
        + MAX_TOKEN_LENGTH
        + MAX_ID_LENGTH
        + MIN_PADDING_LENGTH;
}

impl HandshakeRequest {
    pub(crate) fn encode(&self) -> [u8; Self::BYTESIZE] {
        let mut buf = [0; Self::BYTESIZE];
        let mut pos: usize = 0;

        let mut append = |pos: &mut usize, s: &str| {
            let start = *pos;
            let end = start
                .checked_add(s.len())
                .unwrap_or_else(|| unreachable!("bug: failed to encode HandshakeRequest"));
            buf.get_mut(start..end)
                .unwrap_or_else(|| unreachable!("bug: failed to encode HandshakeRequest"))
                .copy_from_slice(s.as_bytes());
            *pos = end;
        };

        append(&mut pos, START_LINE);
        append(&mut pos, "\r\n");

        append(&mut pos, HOST_PREFIX);
        append(&mut pos, self.host.as_str());
        append(&mut pos, "\r\n");

        append(&mut pos, TOKEN_PREFIX);
        append(&mut pos, self.token.as_str());
        append(&mut pos, "\r\n");

        append(&mut pos, ID_PREFIX);
        append(&mut pos, self.id.as_str());
        append(&mut pos, "\r\n");

        append(&mut pos, CONNECTION_UPGRADE_HEADER);
        append(&mut pos, "\r\n");

        append(&mut pos, UPGRADE_MPCLIPBOARD_RAW_HEADER);
        append(&mut pos, "\r\n");

        append(&mut pos, PADDING_PREFIX);
        #[expect(clippy::arithmetic_side_effects)]
        let padding_len = Self::BYTESIZE - 4 - pos;
        for _ in 0..padding_len {
            append(&mut pos, "P");
        }
        append(&mut pos, "\r\n");

        append(&mut pos, "\r\n");

        // assert_eq!(pos, Self::BYTESIZE);
        buf
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        MAX_HOST_PORT_LENGTH, MAX_ID_LENGTH, MAX_TOKEN_LENGTH, NonEmptyInlineString,
        handshake_request::HandshakeRequest,
    };

    #[test]
    fn test_encode_min() {
        let min = HandshakeRequest {
            host: NonEmptyInlineString::new("h").unwrap(),
            token: NonEmptyInlineString::new("t").unwrap(),
            id: NonEmptyInlineString::new("i").unwrap(),
        };

        let expected = [
            "GET / HTTP/1.1\r\n",
            "Host: h\r\n",
            "Token: t\r\n",
            "ID: i\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "Padding: PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP\r\n",
            "\r\n"
        ].join("");

        assert_eq!(
            core::str::from_utf8(&min.encode()).map(|s| s.to_string()),
            Ok(expected)
        );
    }

    #[test]
    fn test_encode_max() {
        let max = HandshakeRequest {
            host: NonEmptyInlineString::new(&"h".repeat(MAX_HOST_PORT_LENGTH)).unwrap(),
            token: NonEmptyInlineString::new(&"t".repeat(MAX_TOKEN_LENGTH)).unwrap(),
            id: NonEmptyInlineString::new(&"i".repeat(MAX_ID_LENGTH)).unwrap(),
        };

        let expected = [
            "GET / HTTP/1.1\r\n",
            "Host: hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh\r\n",
            "Token: tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt\r\n",
            "ID: iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "Padding: P\r\n",
            "\r\n"
        ].join("");

        assert_eq!(
            core::str::from_utf8(&max.encode()).map(|s| s.to_string()),
            Ok(expected)
        );
    }
}
