use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HandshakeRequest, HostPort, ID, ID_PREFIX, START_LINE,
    TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    http_lines_reader::{HttpLinesParser, HttpLinesReader, HttpLinesReaderError},
    strip_prefix_ignore_ascii_case,
};

#[must_use]
#[derive(Debug, Clone, Copy)]
struct HandshakeRequestParser {
    seen_start_line: bool,
    host: Option<HostPort>,
    token: Option<Token>,
    id: Option<ID>,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
}

impl HttpLinesParser for HandshakeRequestParser {
    type Output = HandshakeRequest;
    type Error = HandshakeRequestParserError;

    fn new() -> Self {
        Self {
            seen_start_line: false,
            host: None,
            token: None,
            id: None,
            seen_connection_upgrade: false,
            seen_upgrade_mpclipboard_raw: false,
        }
    }

    fn line_received(&mut self, line: &str) -> Result<(), Self::Error> {
        if line.starts_with(START_LINE) {
            self.seen_start_line = true;
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, HOST_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.host =
                Some(HostPort::new(value).map_err(|_| HandshakeRequestParserError::MalformedHost)?);
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, TOKEN_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.token =
                Some(Token::new(value).map_err(|_| HandshakeRequestParserError::MalformedToken)?);
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, ID_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.id = Some(ID::new(value).map_err(|_| HandshakeRequestParserError::MalformedId)?);
        } else if strip_prefix_ignore_ascii_case(line, CONNECTION_UPGRADE_HEADER) == Some("\r\n") {
            self.seen_connection_upgrade = true;
        } else if strip_prefix_ignore_ascii_case(line, UPGRADE_MPCLIPBOARD_RAW_HEADER)
            == Some("\r\n")
        {
            self.seen_upgrade_mpclipboard_raw = true;
        }

        Ok(())
    }

    fn try_finish(&self) -> Option<Self::Output> {
        if self.seen_start_line
            && let Some(host) = self.host
            && let Some(token) = self.token
            && let Some(id) = self.id
            && self.seen_connection_upgrade
            && self.seen_upgrade_mpclipboard_raw
        {
            Some(HandshakeRequest { host, token, id })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRequestParserError {
    MalformedHost,
    MalformedToken,
    MalformedId,
}

impl core::fmt::Display for HandshakeRequestParserError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MalformedHost => f.write_str("malformed Host header"),
            Self::MalformedToken => f.write_str("malformed Token header"),
            Self::MalformedId => f.write_str("malformed ID header"),
        }
    }
}

impl core::error::Error for HandshakeRequestParserError {}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct HandshakeRequestReader {
    inner: HttpLinesReader<HandshakeRequestParser, { HandshakeRequest::BYTESIZE }>,
}

impl HandshakeRequestReader {
    pub fn new() -> Self {
        Self {
            inner: HttpLinesReader::new(HandshakeRequestParser::new()),
        }
    }

    pub fn received(
        &mut self,
        data: &[u8],
    ) -> Result<(usize, Option<HandshakeRequest>), HttpLinesReaderError<HandshakeRequestParserError>>
    {
        self.inner.received(data)
    }
}

impl Default for HandshakeRequestReader {
    fn default() -> Self {
        Self::new()
    }
}
