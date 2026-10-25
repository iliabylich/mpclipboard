use crate::{EventLoopWithTimer, RegisteredFd, Wants};
use core::time::Duration;
use std::{
    io::Write,
    os::{fd::OwnedFd, unix::net::UnixStream},
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetDeadlineAction {
    Keep,
    SetIn(u8),
    Repeat,
    Clear,
}

impl SetDeadlineAction {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputAction {
    Idle,
    Write,
}

impl InputAction {
    fn parse(c: char) -> Self {
        match c {
            '-' => Self::Idle,
            'w' => Self::Write,
            _ => panic!("unknown char: '{c}'"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Actions {
    set_deadline: Vec<SetDeadlineAction>,
    input: Vec<InputAction>,
}

impl Actions {
    fn parse(set_deadline: &str, input: &str) -> Self {
        let set_deadline = set_deadline.chars().map(SetDeadlineAction::parse).collect();
        let input = input.chars().map(InputAction::parse).collect();
        Self {
            set_deadline,
            input,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Expectation {
    Yes,
    No,
}

impl Expectation {
    fn parse(c: char) -> Self {
        match c {
            '1' => Self::Yes,
            '-' => Self::No,
            _ => panic!("unknown char: '{c}'"),
        }
    }

    fn render(self) -> char {
        match self {
            Self::Yes => '1',
            Self::No => '-',
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Expectations {
    pub(crate) ticks: Vec<Expectation>,
    pub(crate) deadlines: Vec<Expectation>,
    pub(crate) reads: Vec<Expectation>,
}

impl Expectations {
    fn parse(ticks: &str, deadlines: &str, reads: &str) -> Self {
        let ticks = ticks.chars().map(Expectation::parse).collect();
        let deadlines = deadlines.chars().map(Expectation::parse).collect();
        let reads = reads.chars().map(Expectation::parse).collect();

        Self {
            ticks,
            deadlines,
            reads,
        }
    }

    pub(crate) fn render(self) -> String {
        fn line(items: &[Expectation]) -> String {
            items.iter().copied().map(Expectation::render).collect()
        }

        [line(&self.ticks), line(&self.deadlines), line(&self.reads)].join("\n")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MarbleSet {
    actions: Actions,
    expectations: Expectations,
    len: usize,
}

impl MarbleSet {
    pub(crate) fn parse(
        set_deadline: &str,
        input: &str,
        ticks: &str,
        deadlines: &str,
        reads: &str,
    ) -> Self {
        let len = set_deadline.len();

        macro_rules! validate {
            ($value:expr) => {
                let name = stringify!($value);
                assert!($value.is_ascii(), "non-ascii marble: {:?}", $value);
                assert_eq!(
                    $value.len(),
                    len,
                    "length mismatch: {name} has {} frames vs {len}",
                    $value.len()
                );
            };
        }

        validate!(set_deadline);
        validate!(input);
        validate!(ticks);
        validate!(deadlines);
        validate!(reads);

        Self {
            actions: Actions::parse(set_deadline, input),
            expectations: Expectations::parse(ticks, deadlines, reads),
            len,
        }
    }

    pub(crate) fn run(self) -> (Expectations, Actuals) {
        const FRAME: Duration = Duration::from_millis(100);
        const READABLE_KEY: usize = 0;

        let mut event_loop = EventLoopWithTimer::new().unwrap();
        let t0 = Instant::now();
        let middle_of = |frame: usize| t0 + FRAME * u32::try_from(frame).unwrap();

        let (reader, mut writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        let reader = event_loop
            .register(OwnedFd::from(reader), READABLE_KEY, Wants::Read)
            .unwrap();

        let mut actuals = Actuals::new(self.len);
        let mut current = None;
        let mut last = None;

        for frame in 0..self.len {
            match self.actions.set_deadline[frame] {
                SetDeadlineAction::Keep => {}
                SetDeadlineAction::SetIn(n) => {
                    current = Some(middle_of(frame + n as usize));
                    last = current;
                }
                SetDeadlineAction::Repeat => {
                    current = Some(last.expect(
                        "can't repeat deadline that hasn't been set yet, check your deadline marble string",
                    ));
                }
                SetDeadlineAction::Clear => current = None,
            }
            event_loop.set_deadline(current).unwrap();

            match self.actions.input[frame] {
                InputAction::Idle => {}
                InputAction::Write => writer.write_all(&[0]).unwrap(),
            }

            let end = middle_of(frame) + FRAME / 2;
            while let Some(remaining) = end.checked_duration_since(Instant::now()) {
                let polled = event_loop.wait(Some(remaining)).unwrap();
                if polled.ticked {
                    actuals.ticks[frame] += 1;
                }
                if polled.deadline_expired {
                    actuals.deadlines[frame] += 1;
                    current = None;
                }
                let mut readable = false;
                for (key, fd) in polled.fds() {
                    assert_eq!(key, READABLE_KEY);
                    readable |= fd.readable;
                }
                if readable {
                    actuals.reads[frame] += 1;
                    reader.drain();
                }

                event_loop.set_deadline(current).unwrap();
            }
        }

        (self.expectations, actuals)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Actuals {
    pub(crate) ticks: Vec<u8>,
    pub(crate) deadlines: Vec<u8>,
    pub(crate) reads: Vec<u8>,
}

impl Actuals {
    fn new(len: usize) -> Self {
        Self {
            ticks: vec![0; len],
            deadlines: vec![0; len],
            reads: vec![0; len],
        }
    }

    pub(crate) fn render(self) -> String {
        fn line(items: &[u8]) -> String {
            items
                .iter()
                .copied()
                .map(|count| match count {
                    0 => '-',
                    1..=9 => (count + b'0') as char,
                    _ => '+',
                })
                .collect()
        }

        [line(&self.ticks), line(&self.deadlines), line(&self.reads)].join("\n")
    }
}

impl RegisteredFd {
    fn drain(&self) {
        let mut buf = [0; 64];
        while rustix::io::read(self, &mut buf).is_ok_and(|len| len > 0) {}
    }
}

macro_rules! marbles {
    (
        actions: {
            set_deadlines: $set_deadlines:literal,
            input: $input:literal,
        },
        expectations: {
            ticks: $ticks:literal,
            deadlines: $deadlines:literal,
            reads: $reads:literal,
        }
    ) => {{
        use $crate::marble::MarbleSet;
        let marble = MarbleSet::parse($set_deadlines, $input, $ticks, $deadlines, $reads);
        let (expected, actual) = marble.run();
        let expected = expected.render();
        let actual = actual.render();
        assert_eq!(expected, actual);
    }};
}
pub(crate) use marbles;

#[test]
fn test_parse_full() {
    let scenario = MarbleSet::parse(
        "0123456789-=x",
        "-w-----------",
        "1------------",
        "------1------",
        "------------1",
    );

    assert_eq!(
        scenario,
        MarbleSet {
            actions: Actions {
                set_deadline: vec![
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
                ],
                input: vec![
                    InputAction::Idle,
                    InputAction::Write,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                    InputAction::Idle,
                ],
            },
            expectations: Expectations {
                ticks: vec![
                    Expectation::Yes,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                ],
                deadlines: vec![
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::Yes,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                ],
                reads: vec![
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::No,
                    Expectation::Yes,
                ],
            },
            len: 13
        }
    );
}

#[test]
#[should_panic(expected = "unknown char: 'a'")]
fn test_parse_unknown_deadline_symbol() {
    MarbleSet::parse("-a-", "---", "---", "---", "---");
}

#[test]
#[should_panic(expected = "unknown char: 'r'")]
fn test_parse_unknown_input_symbol() {
    MarbleSet::parse("---", "--r", "---", "---", "---");
}

#[test]
#[should_panic(expected = "unknown char: '0'")]
fn test_parse_unknown_fired_symbol() {
    MarbleSet::parse("---", "---", "---", "0--", "---");
}

#[test]
#[should_panic(expected = "unknown char: '2'")]
fn test_parse_fired_count() {
    MarbleSet::parse("---", "---", "---", "---", "-2-");
}

#[test]
#[should_panic(expected = "length mismatch: reads has 4 frames vs 3")]
fn test_parse_length_mismatch() {
    MarbleSet::parse("---", "---", "---", "---", "----");
}

#[test]
#[should_panic(expected = "can't repeat deadline that hasn't been set yet")]
fn test_repeat_without_deadline() {
    MarbleSet::parse("-=1-", "----", "----", "----", "----").run();
}

#[test]
fn test_render() {
    let rows = Actuals {
        ticks: vec![0, 1, 9, 10],
        deadlines: vec![0; 4],
        reads: vec![2; 4],
    };

    assert_eq!(
        rows.render(),
        ["-19+".to_string(), "----".to_string(), "2222".to_string(),].join("\n")
    );
}
