use crate::marble::parse::MarbleParse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetDeadlineAction {
    Keep,
    SetIn(u8),
    Repeat,
    Clear,
}

impl MarbleParse for SetDeadlineAction {
    type Input = char;

    fn parse(c: char) -> Self {
        match c {
            '-' => Self::Keep,
            '=' => Self::Repeat,
            'x' => Self::Clear,
            '0'..='9' => Self::SetIn(c as u8 - b'0'),
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        Vec::<SetDeadlineAction>::parse("0123456789-=x"),
        vec![
            SetDeadlineAction::SetIn(0),
            SetDeadlineAction::SetIn(1),
            SetDeadlineAction::SetIn(2),
            SetDeadlineAction::SetIn(3),
            SetDeadlineAction::SetIn(4),
            SetDeadlineAction::SetIn(5),
            SetDeadlineAction::SetIn(6),
            SetDeadlineAction::SetIn(7),
            SetDeadlineAction::SetIn(8),
            SetDeadlineAction::SetIn(9),
            SetDeadlineAction::Keep,
            SetDeadlineAction::Repeat,
            SetDeadlineAction::Clear,
        ]
    );
}

#[test]
#[should_panic(expected = "unknown char: 'a'")]
fn test_parse_error() {
    Vec::<SetDeadlineAction>::parse("-a-");
}
