mod deadline;
pub(crate) use deadline::DeadlineExpectation;

mod read;
pub(crate) use read::ReadExpectation;

mod tick;
pub(crate) use tick::TickExpectation;

use crate::marble::Rows;

pub(crate) type Expectations = Rows<TickExpectation, DeadlineExpectation, ReadExpectation>;
