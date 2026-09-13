use anyhow::Result;
use mpclipboard_generic_client::{MPClipboard, Output};
use mpclipboard_shared::REvents;
use rustix::event::{PollFd, PollFlags};
use std::io::BufRead;

const HELP: &str = "Usage:
cargo run --example cli -- <periodically sent text>

Example:

RUST_LOG=info cargo run --example cli -- <id>
";
fn print_help_and_exit() -> ! {
    eprintln!("{HELP}");
    std::process::exit(1);
}

fn main() -> Result<()> {
    let [_, id] = std::env::args()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|_| print_help_and_exit());

    let mut mpclipboard = MPClipboard::new_with_local_config_and_id_override(&id)?;
    // let mut mpclipboard = MPClipboard::new_with_xdg_config()?;
    let mut stdin = std::io::stdin().lock();

    loop {
        let mut fds = [
            PollFd::new(&mpclipboard, PollFlags::IN),
            PollFd::new(&stdin, PollFlags::IN),
        ];
        rustix::event::poll(&mut fds, None)?;
        let mpclipboard_revents = REvents::new(fds[0].revents())?;
        let stdin_revents = REvents::new(fds[1].revents())?;

        assert!(!mpclipboard_revents.writable);
        if mpclipboard_revents.readable {
            if let Some(output) = mpclipboard.read()? {
                match output {
                    Output::ConnectivityChanged { connectivity } => {
                        println!("[c] {connectivity:?}")
                    }
                    Output::NewText { text } => println!("[{text}]"),
                    Output::Both { connectivity, text } => {
                        println!("[c] {connectivity:?}");
                        println!("[{text}]");
                    }
                }
            }
        }

        assert!(!stdin_revents.writable);
        if stdin_revents.readable {
            let mut line = String::new();
            stdin.read_line(&mut line)?;
            let line = line.trim();
            if !line.is_empty() {
                mpclipboard.push_text(line)?;
            }
        }
    }
}
