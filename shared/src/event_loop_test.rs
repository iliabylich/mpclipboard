use crate::marble::marbles;

#[test]
fn test_marbles() {
    marbles! {
        actions: {
            set_deadlines: "-3----=-----5-x------",
            input:         "--------w------------",
        },
        expectations: {
            ticks:         "1---------1---------1",
            deadlines:     "----1-1--------------",
            reads:         "--------1------------",
        }
    }
}
