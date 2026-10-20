use crate::connection::{
    actions::{ReadDnsResponseError, StartResolvingError, WriteDnsRequestError},
    state::Http3StateError,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionError {
    #[error("failed to start resolving: {0}")]
    StartResolving(#[from] StartResolvingError),
    #[error("failed to write DNS request: {0}")]
    WriteDnsRequest(#[from] WriteDnsRequestError),
    #[error("failed to read DNS response: {0}")]
    ReadDnsResponse(#[from] ReadDnsResponseError),
    #[error("{0}")]
    Http3(#[from] Http3StateError),
}
