use anyhow::{Context as _, Result};
use mpclipboard_generic_client::{Config, Context, MPClipboard, Output};
use polling::{Event, Events, PollMode, Poller};
use std::os::fd::AsRawFd as _;

const HELP: &str = "Usage:
cargo run --example cli -- <URI> <token> <name> <periodically sent text>

Example:

RUST_LOG=info cargo run --example cli -- ws://127.0.0.1:3000 sekret user-no-42
";
fn print_help_and_exit() -> ! {
    log::error!("{HELP}");
    std::process::exit(1);
}

const MPCLIPBOARD: u32 = 1;
const TIMER: u32 = 2;

fn main() -> Result<()> {
    MPClipboard::init()?;

    let [_, uri, token, name, flood] = std::env::args()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|_| print_help_and_exit());

    let config = Config::new(&uri, token, name)?;
    let context = Context::new(config)?;

    let mut mpclipboard = MPClipboard::new(context);

    let poller = setup_external_event_loop(mpclipboard.as_raw_fd())?;
    let mut tick = 0;

    loop {
        let mut events = Events::new();
        poller.wait(&mut events, None).context("failed to poll")?;

        for event in events.iter() {
            match event.key as u32 {
                MPCLIPBOARD => {
                    if let Some(output) = mpclipboard.read()? {
                        match output {
                            Output::ConnectivityChanged { connectivity } => {
                                log::info!("{connectivity:?}")
                            }
                            Output::NewText { text } => log::info!("[{text}]"),
                        }
                    }
                }
                TIMER => {
                    tick += 1;
                    drain_timer();

                    if tick % 2 == 0 {
                        let _ = mpclipboard.push_text(format!("{flood}-tick-{tick}"))?;
                        // OR
                        // mpclipboard.push_binary(vec![1, 2, 3]);
                    }
                }
                other => unreachable!("unknown event key: {other}"),
            }
        }
    }
}

#[cfg(target_os = "linux")]
static mut TIMERFD: i32 = -1;

fn setup_external_event_loop(mpclipboard_fd: i32) -> Result<Poller> {
    let poller = Poller::new()?;
    (unsafe {
        poller.add_with_mode(
            mpclipboard_fd,
            Event::new(MPCLIPBOARD as usize, true, false),
            PollMode::Level,
        )
    })?;

    #[cfg(target_os = "macos")]
    {
        use polling::os::kqueue::{PollerKqueueExt, Timer};
        use std::time::Duration;
        poller.add_filter(
            Timer {
                id: TIMER as usize,
                timeout: Duration::from_secs(1),
            },
            TIMER as usize,
            PollMode::Level,
        )?;
    }

    #[cfg(target_os = "linux")]
    {
        use rustix::time::{
            Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
            timerfd_settime,
        };
        use std::os::fd::IntoRawFd;

        let timerfd = timerfd_create(TimerfdClockId::Monotonic, TimerfdFlags::NONBLOCK)
            .expect("bug: failed to create timerfd");

        timerfd_settime(
            &timerfd,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
            },
        )
        .expect("bug: failed to configure timer");

        unsafe {
            poller
                .add_with_mode(
                    &timerfd,
                    Event::new(TIMER as usize, true, false),
                    PollMode::Level,
                )
                .expect("bug: failed to add timer to epoll")
        };

        unsafe { TIMERFD = timerfd.into_raw_fd() }
    }

    Ok(poller)
}

#[cfg(target_os = "macos")]
fn drain_timer() {}

#[cfg(target_os = "linux")]
fn drain_timer() {
    use std::os::fd::BorrowedFd;

    let mut buf = [0_u8; 8];
    let bytes_read = rustix::io::read(unsafe { BorrowedFd::borrow_raw(TIMERFD) }, &mut buf)
        .expect("bug: failed to read from timer");
    assert_eq!(bytes_read, 8);
}
