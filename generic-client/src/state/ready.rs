use crate::{
    event_loop::{EventLoopAwareWebSocket, ReadResult, WriteResult},
    state::{Disconnected, State},
};
use clip::Clip;
use tungstenite::Message;

pub(crate) struct Ready(pub(crate) EventLoopAwareWebSocket);

impl Ready {
    pub(crate) fn read_write(
        mut self,
        readable: bool,
        writable: bool,
        pending_message_to_send: &mut Option<Message>,
        write_blocked: &mut bool,
    ) -> (State, Option<Clip>) {
        if writable
            && self.0.can_write()
            && let Some(message) = pending_message_to_send.take()
        {
            match self.0.write(message) {
                Ok(WriteResult::Done) => {}
                Ok(WriteResult::WouldBlock) => {
                    *write_blocked = true;
                }
                Ok(WriteResult::DeadEnd) => {
                    log::error!("writer got DeadEnd (connection closed)");
                    return (State::Disconnected(Disconnected), None);
                }
                Ok(WriteResult::QueueIsFull(write_me_back)) => {
                    *pending_message_to_send = Some(write_me_back);
                }
                Err(err) => {
                    log::error!("{err:?}");
                    return (State::Disconnected(Disconnected), None);
                }
            };
        }

        let mut clip = None;

        if readable && self.0.can_read() {
            match self.0.read() {
                Ok(ReadResult::Message(message)) => {
                    log::trace!("{message:?}");
                    if let Message::Binary(bytes) = message {
                        match Clip::decode(bytes.into()) {
                            Ok(decoded) => clip = Some(decoded),
                            Err(err) => {
                                log::error!("{err:?}");
                                return (State::Disconnected(Disconnected), None);
                            }
                        }
                    }
                }
                Ok(ReadResult::WouldBlock) => {
                    log::trace!("nothing to read");
                }
                Err(err) => {
                    log::error!("{err:?}");
                    return (State::Disconnected(Disconnected), None);
                }
            };
        }

        (State::Ready(Ready(self.0)), clip)
    }
}
