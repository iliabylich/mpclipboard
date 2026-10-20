use crate::{ID, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub token: Token,
    pub id: ID,
}
