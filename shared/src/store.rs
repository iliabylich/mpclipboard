use crate::message::Message;

#[must_use]
pub struct Store {
    text: Option<Message>,
}

impl Store {
    pub const fn empty() -> Self {
        Self { text: None }
    }

    #[must_use]
    pub const fn current(&self) -> Option<Message> {
        self.text
    }

    #[must_use]
    pub fn add(&mut self, message: Message) -> bool {
        let do_update = self.text.is_none()
            || self
                .text
                .as_ref()
                .is_some_and(|current| message.text_as_bytes() != current.text_as_bytes());

        if do_update {
            self.text = Some(message);
        }

        do_update
    }
}
