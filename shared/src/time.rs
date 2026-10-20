#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now(u64);

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Past(u64);

impl Now {
    pub(crate) const fn new(seconds: u64) -> Self {
        Self(seconds)
    }

    pub const fn as_past(self) -> Past {
        Past(self.0)
    }

    #[must_use]
    pub fn seconds_since(self, past: Past) -> u64 {
        self.0
            .checked_sub(past.0)
            .unwrap_or_else(|| unreachable!("time goes backwards"))
    }
}

impl core::fmt::Display for Now {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Display for Past {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Now;

    #[test]
    fn test_seconds_since() {
        let past = Now::new(3).as_past();
        assert_eq!(Now::new(3).seconds_since(past), 0);
        assert_eq!(Now::new(10).seconds_since(past), 7);
    }

    #[test]
    #[should_panic(expected = "time goes backwards")]
    fn test_time_goes_backwards() {
        let past = Now::new(10).as_past();
        let _ = Now::new(3).seconds_since(past);
    }
}
