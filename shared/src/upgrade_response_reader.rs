use crate::{
    CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    http_lines_reader::{HttpLinesParser, HttpLinesReader, HttpLinesReaderError},
    message::Message,
    strip_prefix_ignore_ascii_case,
};

#[must_use]
#[derive(Debug, Clone, Copy)]
struct UpgradeResponseParser {
    seen_start_line: bool,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
}

impl HttpLinesParser for UpgradeResponseParser {
    type Output = ();
    type Error = UpgradeResponseParserError;

    fn new() -> Self {
        Self {
            seen_start_line: false,
            seen_connection_upgrade: false,
            seen_upgrade_mpclipboard_raw: false,
        }
    }

    fn line_received(&mut self, line: &str) -> Result<(), Self::Error> {
        if line.starts_with("HTTP/1.1 101 Switching Protocols") {
            self.seen_start_line = true;
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
        if self.seen_start_line && self.seen_connection_upgrade && self.seen_upgrade_mpclipboard_raw
        {
            Some(())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeResponseParserError {}

impl core::fmt::Display for UpgradeResponseParserError {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {}
    }
}

impl core::error::Error for UpgradeResponseParserError {}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct UpgradeResponseReader {
    inner: HttpLinesReader<UpgradeResponseParser, { Message::BYTESIZE }>,
}

impl UpgradeResponseReader {
    pub fn new() -> Self {
        Self {
            inner: HttpLinesReader::new(UpgradeResponseParser::new()),
        }
    }

    pub fn received(
        &mut self,
        data: &[u8],
    ) -> Result<(usize, Option<()>), HttpLinesReaderError<UpgradeResponseParserError>> {
        self.inner.received(data)
    }
}

impl Default for UpgradeResponseReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upgrade_response::UpgradeResponse;

    #[test]
    fn fragmented_response_is_decoded() {
        let mut reader = UpgradeResponseReader::new();

        for (index, byte) in UpgradeResponse::BYTES.iter().enumerate() {
            let (consumed, output) = reader.received(core::slice::from_ref(byte)).unwrap();
            assert_eq!(consumed, 1);
            assert_eq!(output.is_some(), index + 1 == UpgradeResponse::BYTESIZE);
        }
    }

    #[test]
    fn bytes_after_response_are_left_for_the_caller() {
        let mut data = Vec::from(UpgradeResponse::BYTES);
        data.extend_from_slice(b"raw protocol bytes");
        let mut reader = UpgradeResponseReader::new();

        let (consumed, output) = reader.received(&data).unwrap();

        assert!(output.is_some());
        assert_eq!(consumed, UpgradeResponse::BYTESIZE);
        assert_eq!(&data[consumed..], b"raw protocol bytes");
    }

    #[test]
    fn oversized_incomplete_response_is_rejected() {
        let response_without_end = UpgradeResponse::BYTES
            .strip_suffix(b"\r\n")
            .unwrap_or_else(|| unreachable!());
        let mut data = Vec::from(response_without_end);
        while data.len() <= Message::BYTESIZE {
            data.extend_from_slice(b"X: y\r\n");
        }
        data.extend_from_slice(b"\r\n");
        let mut reader = UpgradeResponseReader::new();

        assert!(reader.received(&data).is_err());
    }
}
