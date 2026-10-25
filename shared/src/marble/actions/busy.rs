use crate::marble::parse::MarbleParse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BusyAction {
    RunNormally,
    SimulateBusy,
}

impl MarbleParse for BusyAction {
    type Input = char;

    fn parse(c: char) -> Self {
        match c {
            '-' => Self::RunNormally,
            'b' => Self::SimulateBusy,
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        Vec::<BusyAction>::parse("-b"),
        vec![BusyAction::RunNormally, BusyAction::SimulateBusy]
    );
}

#[test]
#[should_panic(expected = "unknown char: 's'")]
fn test_parse_error() {
    Vec::<BusyAction>::parse("-s-");
}
