use crate::marble::parse::MarbleParse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputAction {
    Idle,
    Write,
}

impl MarbleParse for InputAction {
    type Input = char;

    fn parse(c: char) -> Self {
        match c {
            '-' => Self::Idle,
            'w' => Self::Write,
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        Vec::<InputAction>::parse("-w"),
        vec![InputAction::Idle, InputAction::Write]
    );
}

#[test]
#[should_panic(expected = "unknown char: 'r'")]
fn test_parse_error() {
    Vec::<InputAction>::parse("--r");
}
