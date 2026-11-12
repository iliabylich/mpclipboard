use crate::Connectivity;
use externally_driven_async_rt::State;
use mpclipboard_shared2::Message;

#[derive(Debug)]
pub(crate) struct LoopState {
    connectivity: Option<Connectivity>,
    received: Option<Message>,
}

impl LoopState {
    pub(crate) const fn new() -> Self {
        Self {
            connectivity: None,
            received: None,
        }
    }

    pub(crate) const fn take_updates(&mut self) -> (Option<Connectivity>, Option<Message>) {
        (self.connectivity.take(), self.received.take())
    }
}

impl State for LoopState {
    type Event = LoopEvent;

    fn apply(&mut self, event: LoopEvent) {
        match event {
            LoopEvent::Connectivity(connectivity) => self.connectivity = Some(connectivity),
            LoopEvent::Received(message) => self.received = Some(message),
        }
    }
}

#[expect(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum LoopEvent {
    Connectivity(Connectivity),
    Received(Message),
}

#[cfg(test)]
mod tests {
    use super::{LoopEvent, LoopState};
    use crate::Connectivity;
    use externally_driven_async_rt::State;
    use mpclipboard_shared2::Message;

    #[test]
    fn test_initially_empty() {
        let mut state = LoopState::new();
        assert_eq!(state.take_updates(), (None, None));
    }

    #[test]
    fn test_change_is_reported_once() {
        let mut state = LoopState::new();
        state.apply(LoopEvent::Connectivity(Connectivity::Connecting));
        assert_eq!(state.take_updates(), (Some(Connectivity::Connecting), None));
        assert_eq!(state.take_updates(), (None, None));
    }

    #[test]
    fn test_last_connectivity_wins() {
        let mut state = LoopState::new();
        state.apply(LoopEvent::Connectivity(Connectivity::Connected));
        state.apply(LoopEvent::Connectivity(Connectivity::Disconnected));
        state.apply(LoopEvent::Connectivity(Connectivity::Connected));
        assert_eq!(state.take_updates(), (Some(Connectivity::Connected), None));
        assert_eq!(state.take_updates(), (None, None));
    }

    #[test]
    fn test_last_received_wins() {
        let mut state = LoopState::new();
        state.apply(LoopEvent::Received(Message::truncate("a").unwrap()));
        state.apply(LoopEvent::Received(Message::truncate("b").unwrap()));
        assert_eq!(
            state.take_updates(),
            (None, Some(Message::truncate("b").unwrap()))
        );
        assert_eq!(state.take_updates(), (None, None));
    }
}
