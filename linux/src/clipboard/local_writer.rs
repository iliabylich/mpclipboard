use anyhow::{Context as _, Result};

pub(crate) struct LocalWriter;

impl LocalWriter {
    pub(crate) fn write(text: &str) {
        if let Err(err) = Self::try_write_text(text) {
            log::error!(target: "LocalWriter", "{err:?}");
        }
    }

    fn try_write_text(text: &str) -> Result<()> {
        use wl_clipboard_rs::copy::{MimeType, Options, Source, copy};

        let options = Options::default();
        copy(
            options,
            Source::Bytes(text.to_string().into_bytes().into_boxed_slice()),
            MimeType::Text,
        )
        .context("failed to write contents to clipboard")?;
        Ok(())
    }
}
