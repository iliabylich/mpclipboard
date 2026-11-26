use crate::Connectivity;
use mpclipboard_shared::Message;

#[derive(Debug)]
#[must_use]
pub struct Output {
    pub connectivity: Option<Connectivity>,
    pub text: Option<Message>,
}
