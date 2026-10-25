use crate::marble::{MarbleRender, parse::MarbleParse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickExpectation {
    NoTick,
    TickAt(u8),
}

impl MarbleParse for TickExpectation {
    type Input = char;

    fn parse(c: char) -> Self {
        match c {
            '-' => Self::NoTick,
            '0'..='9' => Self::TickAt(c as u8 - b'0'),
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

impl MarbleRender for TickExpectation {
    type Output = char;

    fn render(&self) -> char {
        match self {
            Self::NoTick => '-',
            Self::TickAt(now) => (now + b'0') as char,
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        Vec::<TickExpectation>::parse("0-57"),
        vec![
            TickExpectation::TickAt(0),
            TickExpectation::NoTick,
            TickExpectation::TickAt(5),
            TickExpectation::TickAt(7)
        ]
    );
}

#[test]
#[should_panic(expected = "unknown char: '+'")]
fn test_parse_error() {
    Vec::<TickExpectation>::parse("-+-");
}
