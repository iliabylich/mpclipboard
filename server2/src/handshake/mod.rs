use crate::fixed_str::FixedStr;

mod parse;
pub(crate) use parse::parse;

pub(crate) type Token = FixedStr<20>;
pub(crate) type ID = FixedStr<20>;
