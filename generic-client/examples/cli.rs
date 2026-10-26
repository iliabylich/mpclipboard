use mpclipboard_generic_client::{MPClipboard, Output};
use rustix::event::{PollFd, PollFlags};
use std::io::BufRead;

fn main() {
    let mut mpclipboard = MPClipboard::new_from_file().unwrap();
    let mut stdin = std::io::stdin().lock();

    loop {
        let mut fds = [
            PollFd::new(&mpclipboard, PollFlags::IN),
            PollFd::new(&stdin, PollFlags::IN),
        ];
        rustix::event::poll(&mut fds, None).unwrap();
        let [mpclipboard_revents, stdin_revents] = fds.each_ref().map(PollFd::revents);
        for revents in [mpclipboard_revents, stdin_revents] {
            assert!(
                !revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL),
                "got revents {revents:?}"
            );
        }

        if mpclipboard_revents.contains(PollFlags::IN) {
            let Output { connectivity, text } = mpclipboard.read().unwrap();
            if let Some(connectivity) = connectivity {
                println!("[c] {connectivity:?}");
            }
            if let Some(text) = text {
                println!("[{text}]");
            }
        }

        if stdin_revents.contains(PollFlags::IN) {
            let mut line = String::new();
            stdin.read_line(&mut line).unwrap();
            let line = line.trim();
            if !line.is_empty() {
                mpclipboard.push_text(line).unwrap();
            }
        }
    }
}
