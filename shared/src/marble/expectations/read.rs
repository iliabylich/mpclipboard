use crate::marble::{MarbleRender, parse::MarbleParse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadExpectation {
    Read,
    NoRead,
}

impl MarbleParse for ReadExpectation {
    type Input = char;

    fn parse(c: char) -> Self {
        match c {
            '1' => Self::Read,
            '-' => Self::NoRead,
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

impl MarbleRender for ReadExpectation {
    type Output = char;

    fn render(&self) -> char {
        match self {
            Self::Read => '1',
            Self::NoRead => '-',
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        Vec::<ReadExpectation>::parse("-1"),
        vec![ReadExpectation::NoRead, ReadExpectation::Read]
    );
}

#[test]
#[should_panic(expected = "unknown char: '2'")]
fn test_parse_error() {
    Vec::<ReadExpectation>::parse("-2-");
}
