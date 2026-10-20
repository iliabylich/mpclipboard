mod start_resolving;
pub(crate) use start_resolving::{StartResolvingError, start_resolving};

mod write_dns_request;
pub(crate) use write_dns_request::{WriteDnsRequestError, write_dns_request};

mod read_dns_response;
pub(crate) use read_dns_response::{ReadDnsResponseError, read_dns_response};

mod connect;
pub(crate) use connect::{ConnectError, connect};
