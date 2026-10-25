mod tick;
pub(crate) use tick::ActualTick;

mod occurrence;
pub(crate) use occurrence::ActualOccurrence;

use crate::marble::{MarbleRender, Rows};

pub(crate) type Actuals = Rows<ActualTick, ActualOccurrence, ActualOccurrence>;

impl Actuals {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            ticks: vec![ActualTick::NoTick; len],
            deadlines: vec![ActualOccurrence(0); len],
            reads: vec![ActualOccurrence(0); len],
        }
    }
}

#[test]
fn test_render() {
    let rows = Actuals {
        ticks: vec![
            ActualTick::NoTick,
            ActualTick::TickAt(3),
            ActualTick::Multiple,
            ActualTick::TickAt(9),
        ],
        deadlines: vec![ActualOccurrence(0); 4],
        reads: vec![ActualOccurrence(2); 4],
    };

    assert_eq!(
        rows.render(),
        ["-3+9".to_string(), "----".to_string(), "2222".to_string()].join("\n")
    );
}
