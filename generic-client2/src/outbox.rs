use core::cell::Cell;
use mpclipboard_shared2::Message;

#[derive(Debug, Default)]
pub(crate) struct Outbox(Cell<Option<Message>>);

impl Outbox {
    pub(crate) fn put(&self, message: Message) {
        self.0.set(Some(message));
    }

    pub(crate) fn take(&self) -> Option<Message> {
        self.0.take()
    }
}
