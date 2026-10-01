#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::future_not_send)]
#![doc = include_str!("../README.md")]

use anyhow::Result;
use clipboard::LocalClipboared;
use dbus::{DBus, DBusQueue};
use mini_sansio_dbus::messages::org_freedesktop_dbus::Hello;
use mpclipboard::MPClipboard;
use mpclipboard_generic_client::Output;
use revents::REvents;

use crate::tray::Tray;

mod clipboard;
mod dbus;
mod mpclipboard;
mod revents;
mod tray;

fn main() -> Result<()> {
    let mut mpclipboard = MPClipboard::new()?;
    let mut clipboard = LocalClipboared::new()?;

    let mut queue = DBusQueue::new();
    queue.push_without_reply::<Hello>(())?;

    let mut tray = Tray::new(&mut queue)?;

    let mut dbus = DBus::new()?;
    let mut readbuf = [0; 10 * 1_024];

    while !tray.received_exit() {
        let ready = poll(&mpclipboard, &clipboard, &dbus, &mut readbuf, &queue)?;

        if ready.dbus.writable && dbus.wants_write(&mut readbuf, &queue)? {
            dbus.write(&mut readbuf, &mut queue)?;
        }

        if ready.dbus.readable
            && dbus.wants_read(&mut readbuf)?
            && let Some(message) = dbus.read(&mut readbuf, &queue)?
        {
            tray.handle(message, &mut queue)?;
        }

        if ready.mpclipboard.readable {
            let output = mpclipboard.read()?;
            log::trace!("{output:?}");
            let Output { connectivity, text } = output;

            if let Some(connectivity) = connectivity {
                tray.set_connectivity(connectivity, &mut queue)?;
            }

            if let Some(text) = text {
                tray.push(format!("R {text}"), &mut queue)?;
                clipboard.offer_text(text)?;
            }
        }

        if ready.clipboard.readable
            && let Some(text) = clipboard.read()?
        {
            log::trace!("Copied: {text:?}");
            tray.push(format!("S {text}"), &mut queue)?;
            mpclipboard.push_text(&text)?;
        }
    }

    Ok(())
}

fn poll(
    mpclipboard: &MPClipboard,
    clipboard: &LocalClipboared,
    dbus: &DBus,
    readbuf: &mut [u8],
    queue: &DBusQueue,
) -> Result<PollResult> {
    let mut pollfds = [
        dbus.as_pollfd(readbuf, queue)?,
        mpclipboard.as_pollfd(),
        clipboard.as_pollfd(),
    ];
    rustix::event::poll(&mut pollfds, None)?;

    Ok(PollResult {
        dbus: REvents::new("DBus", pollfds[0].revents())?,
        mpclipboard: REvents::new("MPClipboard", pollfds[1].revents())?,
        clipboard: REvents::new("LocalClipboard", pollfds[2].revents())?,
    })
}

struct PollResult {
    dbus: REvents,
    mpclipboard: REvents,
    clipboard: REvents,
}
