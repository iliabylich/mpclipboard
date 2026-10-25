use crate::marble::{MarbleRender, parse::MarbleParse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeadlineExpectation {
    Deadline,
    NoDeadline,
}

impl MarbleParse for DeadlineExpectation {
    type Input = char;

    fn parse(c: char) -> Self {
        match c {
            '1' => Self::Deadline,
            '-' => Self::NoDeadline,
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

impl MarbleRender for DeadlineExpectation {
    type Output = char;

    fn render(&self) -> char {
        match self {
            Self::Deadline => '1',
            Self::NoDeadline => '-',
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        Vec::<DeadlineExpectation>::parse("-1"),
        vec![
            DeadlineExpectation::NoDeadline,
            DeadlineExpectation::Deadline
        ]
    );
}

#[test]
#[should_panic(expected = "unknown char: '0'")]
fn test_parse_error() {
    Vec::<DeadlineExpectation>::parse("0--");
}
