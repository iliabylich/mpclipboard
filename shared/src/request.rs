use crate::{ID, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub token: Token,
    pub id: ID,
}
