use anyhow::{Context as _, Result};
use mpclipboard_generic_client::{Config, ConfigReadOption, Context, MPClipboard, Output};
use std::os::fd::AsRawFd;
use tokio::io::unix::AsyncFd;

pub(crate) struct MPClipboardStream {
    mpclipboard: MPClipboard,
    fd: AsyncFd<i32>,
}

const CONFIG_READ_OPTION: ConfigReadOption = if cfg!(debug_assertions) {
    ConfigReadOption::FromLocalFile
} else {
    ConfigReadOption::FromXdgConfigDir
};

impl MPClipboardStream {
    pub(crate) fn init() -> Result<()> {
        MPClipboard::init()
    }

    pub(crate) fn new() -> Result<Self> {
        let config = Config::read(CONFIG_READ_OPTION)?;
        let context = Context::new(config)?;

        let mpclipboard = MPClipboard::new(context);
        let fd = AsyncFd::new(mpclipboard.as_raw_fd()).context("failed to construct AsyncFd")?;

        Ok(Self { mpclipboard, fd })
    }

    pub(crate) async fn read(&mut self) -> Result<Option<Output>> {
        let mut guard = self.fd.readable().await.context("failed to wait")?;
        guard.clear_ready();

        Ok(self.mpclipboard.read())
    }

    pub(crate) fn push_text(&mut self, text: String) -> bool {
        self.mpclipboard.push_text(text)
    }
}
