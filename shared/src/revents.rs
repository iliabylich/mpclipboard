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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct REventsError(pub PollFlags);

impl core::fmt::Display for REventsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "got revents {:?}", self.0)
    }
}

impl core::error::Error for REventsError {}
