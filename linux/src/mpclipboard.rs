use anyhow::Result;
use mpclipboard_generic_client::Output;
use rustix::event::{PollFd, PollFlags};

pub(crate) struct MPClipboard {
    mpclipboard: mpclipboard_generic_client::MPClipboard,
}

impl MPClipboard {
    pub(crate) fn new() -> Result<Self> {
        let mpclipboard = if cfg!(debug_assertions) {
            mpclipboard_generic_client::MPClipboard::new_with_local_config()?
        } else {
            mpclipboard_generic_client::MPClipboard::new_with_xdg_config()?
        };

        Ok(Self { mpclipboard })
    }

    pub(crate) fn as_pollfd(&self) -> PollFd<'_> {
        PollFd::new(&self.mpclipboard, PollFlags::IN)
    }

    pub(crate) fn read(&mut self) -> Result<Output> {
        let output = self.mpclipboard.read()?;
        Ok(output)
    }

    pub(crate) fn push_text(&mut self, text: &str) -> Result<bool> {
        let pushed = self.mpclipboard.push_text(text)?;
        Ok(pushed)
    }
}
