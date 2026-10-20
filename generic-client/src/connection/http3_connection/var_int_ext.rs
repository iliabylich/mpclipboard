use h3wire::H3Code;
use quinn_proto::VarInt;

pub(crate) trait VarIntExt {
    fn from_h3_code(code: H3Code) -> Self;
}

impl VarIntExt for VarInt {
    fn from_h3_code(code: H3Code) -> Self {
        Self::from_u64(code.0).unwrap_or_else(|_| {
            unreachable!("H3Code from h3wire always fits into VarInt: it's either h3wire's own constant or a code that came from quinn")
        })
    }
}
