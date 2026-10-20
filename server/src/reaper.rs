use mpclipboard_shared::{Now, Past};

const REAP_STRANGER_AFTER_IN_SECS: u64 = 3;

pub trait CanBeReaped {
    fn created_at(&self) -> Past;

    fn must_be_reaped(&self, now: Now) -> bool {
        now.seconds_since(self.created_at()) > REAP_STRANGER_AFTER_IN_SECS
    }
}
