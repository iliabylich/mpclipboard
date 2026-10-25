use crate::{EventLoopWithTimer, Now, Wants};
use core::time::Duration;
use std::{
    io::Write,
    os::{fd::OwnedFd, unix::net::UnixStream},
    time::Instant,
};

mod parse;
use parse::MarbleParse;

mod render;
pub(crate) use render::MarbleRender;

mod actions;
use actions::{Actions, BusyAction, InputAction, SetDeadlineAction};

mod expectations;
use expectations::{DeadlineExpectation, Expectations, ReadExpectation, TickExpectation};

mod actuals;
use actuals::Actuals;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Rows<Tick, Deadline, Read> {
    pub(crate) ticks: Vec<Tick>,
    pub(crate) deadlines: Vec<Deadline>,
    pub(crate) reads: Vec<Read>,
}

impl<Tick, Deadline, Read> MarbleRender for Rows<Tick, Deadline, Read>
where
    Tick: MarbleRender<Output = char>,
    Deadline: MarbleRender<Output = char>,
    Read: MarbleRender<Output = char>,
{
    type Output = String;

    fn render(&self) -> Self::Output {
        [
            self.ticks.render(),
            self.deadlines.render(),
            self.reads.render(),
        ]
        .join("\n")
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
        set_deadline: &'static str,
        input: &'static str,
        busy: &'static str,
        ticks: &'static str,
        deadlines: &'static str,
        reads: &'static str,
    ) -> Self {
        let len = set_deadline.len();

        macro_rules! parse {
            ($value:expr) => {{
                assert_eq!(
                    $value.len(),
                    len,
                    "length mismatch: {} has {} frames vs {len}",
                    stringify!($value),
                    $value.len()
                );
                MarbleParse::parse($value)
            }};
        }

        let set_deadline = parse!(set_deadline);
        let input = parse!(input);
        let busy = parse!(busy);
        let ticks = parse!(ticks);
        let deadlines = parse!(deadlines);
        let reads = parse!(reads);

        Self {
            actions: Actions {
                set_deadline,
                input,
                busy,
            },
            expectations: Expectations {
                ticks,
                deadlines,
                reads,
            },
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
        let mut last = None;

        for frame in 0..self.len {
            match self.actions.set_deadline[frame] {
                SetDeadlineAction::Keep => {}
                SetDeadlineAction::SetIn(n) => {
                    let deadline = middle_of(frame + n as usize);
                    last = Some(deadline);
                    event_loop.set_deadline(Some(deadline)).unwrap();
                }
                SetDeadlineAction::Repeat => {
                    let deadline = last.expect(
                        "can't repeat deadline that hasn't been set yet, check your deadline marble string",
                    );
                    event_loop.set_deadline(Some(deadline)).unwrap();
                }
                SetDeadlineAction::Clear => event_loop.set_deadline(None).unwrap(),
            }

            match self.actions.input[frame] {
                InputAction::Idle => {}
                InputAction::Write => writer.write_all(&[0]).unwrap(),
            }

            let end = middle_of(frame) + FRAME / 2;
            match self.actions.busy[frame] {
                BusyAction::RunNormally => {}
                BusyAction::SimulateBusy => {
                    std::thread::sleep(end.saturating_duration_since(Instant::now()));
                    continue;
                }
            }

            while let Some(remaining) = end.checked_duration_since(Instant::now()) {
                let polled = event_loop.wait(Some(remaining)).unwrap();
                let ticked = polled.ticked;
                if polled.deadline_expired {
                    actuals.deadlines[frame].add(1);
                }
                let mut readable = false;
                for (key, fd) in polled.fds() {
                    assert_eq!(key, READABLE_KEY);
                    readable |= fd.readable;
                }
                if readable {
                    actuals.reads[frame].add(1);
                    let mut buf = [0; 64];
                    while rustix::io::read(&reader, &mut buf).is_ok_and(|len| len > 0) {}
                }
                if ticked {
                    let now = event_loop.now().seconds_since(Now::new(0).as_past());
                    let now = u8::try_from(now).unwrap();
                    actuals.ticks[frame].record(now);
                }
            }
        }

        (self.expectations, actuals)
    }
}

macro_rules! marbles {
    (
        actions: {
            set_deadlines: $set_deadlines:literal,
            input: $input:literal,
            busy: $busy:literal,
        },
        expectations: {
            ticks: $ticks:literal,
            deadlines: $deadlines:literal,
            reads: $reads:literal,
        }
    ) => {{
        use $crate::marble::{MarbleRender, MarbleSet};
        let marble = MarbleSet::parse($set_deadlines, $input, $busy, $ticks, $deadlines, $reads);
        let (expected, actual) = marble.run();
        let expected = expected.render();
        let actual = actual.render();
        assert_eq!(expected, actual);
    }};
}
pub(crate) use marbles;

#[test]
fn test_parse_full() {
    let set = MarbleSet::parse("123", "-w-", "-b-", "0--", "--1", "--1");
    assert_eq!(
        set,
        MarbleSet {
            actions: Actions {
                set_deadline: vec![
                    SetDeadlineAction::SetIn(1),
                    SetDeadlineAction::SetIn(2),
                    SetDeadlineAction::SetIn(3),
                ],
                input: vec![InputAction::Idle, InputAction::Write, InputAction::Idle],
                busy: vec![
                    BusyAction::RunNormally,
                    BusyAction::SimulateBusy,
                    BusyAction::RunNormally,
                ],
            },
            expectations: Expectations {
                ticks: vec![
                    TickExpectation::TickAt(0),
                    TickExpectation::NoTick,
                    TickExpectation::NoTick
                ],
                deadlines: vec![
                    DeadlineExpectation::NoDeadline,
                    DeadlineExpectation::NoDeadline,
                    DeadlineExpectation::Deadline
                ],
                reads: vec![
                    ReadExpectation::NoRead,
                    ReadExpectation::NoRead,
                    ReadExpectation::Read
                ],
            },
            len: 3,
        }
    );
}

#[test]
#[should_panic(expected = "length mismatch: reads has 4 frames vs 3")]
fn test_parse_length_mismatch() {
    MarbleSet::parse("---", "---", "---", "---", "---", "----");
}

#[test]
#[should_panic(expected = "can't repeat deadline that hasn't been set yet")]
fn test_repeat_without_deadline() {
    MarbleSet::parse("-=1-", "----", "----", "----", "----", "----").run();
}
