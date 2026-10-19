use mpclipboard_generic_client::{MPClipboard, Output};
use mpclipboard_shared::REvents;
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
        let mpclipboard_revents = REvents::new(fds[0].revents()).unwrap();
        let stdin_revents = REvents::new(fds[1].revents()).unwrap();

        assert!(!mpclipboard_revents.writable);
        if mpclipboard_revents.readable {
            let Output { connectivity, text } = mpclipboard.read().unwrap();
            if let Some(connectivity) = connectivity {
                println!("[c] {connectivity:?}");
            }
            if let Some(text) = text {
                println!("[{text}]");
            }
        }

        assert!(!stdin_revents.writable);
        if stdin_revents.readable {
            let mut line = String::new();
            stdin.read_line(&mut line).unwrap();
            let line = line.trim();
            if !line.is_empty() {
                mpclipboard.push_text(line).unwrap();
            }
        }
    }
}
