use anyhow::Result;
use main_loop::MainLoop;
use mpclipboard::MPClipboardStream;

mod clipboard;
mod main_loop;
mod mpclipboard;
mod tray;

#[tokio::main]
async fn main() -> Result<()> {
    MPClipboardStream::init()?;
    let main_loop = MainLoop::new().await?;
    main_loop.start().await;
    Ok(())
}
