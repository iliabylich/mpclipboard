use rustix::event::PollFlags;

#[derive(Debug, Clone, Copy)]
pub struct REvents {
    pub readable: bool,
    pub writable: bool,
}

impl REvents {
    pub fn new(revents: PollFlags) -> Result<Self, REventsError> {
        if revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL) {
            return Err(REventsError(revents));
        }
        let readable = revents.contains(PollFlags::IN);
        let writable = revents.contains(PollFlags::OUT);
        Ok(Self { readable, writable })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("got revents {0:?}")]
pub struct REventsError(pub PollFlags);
