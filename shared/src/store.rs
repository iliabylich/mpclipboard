use crate::Message;
use core::cell::Cell;

#[derive(Debug, Default)]
pub struct Store {
    last: Cell<Option<Message>>,
}

impl Store {
    pub fn add(&self, message: Message) -> bool {
        if self.last.get() == Some(message) {
            return false;
        }
        self.last.set(Some(message));
        true
    }

    pub fn current(&self) -> Option<Message> {
        self.last.get()
    }
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::Message;

    fn message(text: &str) -> Message {
        Message::truncate(text).unwrap()
    }

    #[test]
    fn test_add() {
        let store = Store::default();
        assert!(store.add(message("a")));
        assert!(!store.add(message("a")));
        assert!(store.add(message("b")));
        assert!(store.add(message("a")));
    }

    #[test]
    fn test_current() {
        let store = Store::default();
        assert_eq!(store.current(), None);
        assert!(store.add(message("a")));
        assert_eq!(store.current(), Some(message("a")));
        assert!(!store.add(message("a")));
        assert_eq!(store.current(), Some(message("a")));
        assert!(store.add(message("b")));
        assert_eq!(store.current(), Some(message("b")));
    }
}
