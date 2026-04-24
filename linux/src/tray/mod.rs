mod buffer;
mod line;
mod state;

use anyhow::{Context as _, Result};
use ksni::{Handle, TrayMethods};
use line::Line;
use mpclipboard_generic_client::Connectivity;
use state::TrayState;
use tokio_util::sync::CancellationToken;

pub(crate) struct Tray {
    handle: Handle<TrayState>,
}

impl Tray {
    pub(crate) async fn spawn(token: CancellationToken) -> Result<Self> {
        let state = TrayState::new(token);
        let handle = state.spawn().await.context("failed to spawn Tray")?;
        Ok(Self { handle })
    }

    pub(crate) async fn push_sent(&self, text: &str) {
        self.handle
            .update(|state| state.buffer.push(Line::Sent(text.to_string())))
            .await;
    }

    pub(crate) async fn push_received(&self, text: &str) {
        self.handle
            .update(|state| state.buffer.push(Line::Received(text.to_string())))
            .await;
    }

    pub(crate) async fn set_connectivity(&self, connectivity: Connectivity) {
        self.handle
            .update(|state| state.connectivity = connectivity)
            .await;
    }

    pub(crate) async fn stop(self) {
        log::info!(target: "Tray", "stopping...");
        self.handle.shutdown().await;
    }
}
