use crate::message::Message;

#[must_use]
#[derive(Debug, Default)]
pub struct Store {
    text: Option<Message>,
}

impl Store {
    #[must_use]
    pub const fn current(&self) -> Option<&Message> {
        self.text.as_ref()
    }

    #[must_use]
    pub fn add(&mut self, message: &Message) -> bool {
        let do_update = self.text.as_ref() != Some(message);

        if do_update {
            self.text = Some(message.clone());
        }

        do_update
    }
}
