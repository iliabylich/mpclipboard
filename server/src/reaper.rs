const REAP_STRANGER_AFTER_IN_SECS: u64 = 3;

pub trait CanBeReaped {
    fn created_at(&self) -> u64;

    fn must_be_reaped(&self, now: u64) -> bool {
        let age = now
            .checked_sub(self.created_at())
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        age > REAP_STRANGER_AFTER_IN_SECS
    }
}
