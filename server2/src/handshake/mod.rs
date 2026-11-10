use crate::fixed_str::FixedStr;

mod parse;
pub(crate) use parse::parse;

pub(crate) type Token = FixedStr<20>;
pub(crate) type ID = FixedStr<20>;

pub(crate) const MAX_REQUEST_SIZE: usize = 2048;
const MAX_HEADERS: usize = 32;
const PROTOCOL_VERSION: &str = "6";
const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";
