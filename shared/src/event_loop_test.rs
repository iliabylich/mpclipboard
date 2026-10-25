use crate::marble::marbles;

#[test]
fn test_ticks() {
    marbles! {
        actions: {
            set_deadlines: "---------------------",
            input:         "---------------------",
            busy:          "---------------------",
        },
        expectations: {
            ticks:         "0---------1---------2",
            deadlines:     "---------------------",
            reads:         "---------------------",
        }
    }
}

#[test]
fn test_deadline() {
    marbles! {
        actions: {
            set_deadlines: "-3---------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "----1------",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_deadline_repeated_while_pending() {
    marbles! {
        actions: {
            set_deadlines: "-4-=-------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "-----1-----",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_expired_deadline_set_again() {
    marbles! {
        actions: {
            set_deadlines: "-2--=------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "---11------",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_deadline_moved_later() {
    marbles! {
        actions: {
            set_deadlines: "-25--------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "-------1---",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_deadline_moved_earlier() {
    marbles! {
        actions: {
            set_deadlines: "-8-2-------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "-----1-----",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_deadline_cleared() {
    marbles! {
        actions: {
            set_deadlines: "-3-x2------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "------1----",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_deadline_at_tick() {
    marbles! {
        actions: {
            set_deadlines: "-9---------",
            input:         "-----------",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "----------1",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_deadline_after_tick() {
    marbles! {
        actions: {
            set_deadlines: "-----8---------",
            input:         "---------------",
            busy:          "---------------",
        },
        expectations: {
            ticks:         "0---------1----",
            deadlines:     "-------------1-",
            reads:         "---------------",
        }
    }
}

#[test]
fn test_deadline_after_tick_cleared() {
    marbles! {
        actions: {
            set_deadlines: "-----8---x-----",
            input:         "---------------",
            busy:          "---------------",
        },
        expectations: {
            ticks:         "0---------1----",
            deadlines:     "---------------",
            reads:         "---------------",
        }
    }
}

#[test]
fn test_reads() {
    marbles! {
        actions: {
            set_deadlines: "-4---------",
            input:         "--w--w----w",
            busy:          "-----------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "-----1-----",
            reads:         "--1--1----1",
        }
    }
}

#[test]
fn test_events_while_busy() {
    marbles! {
        actions: {
            set_deadlines: "-2---------",
            input:         "---w-------",
            busy:          "---b-------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "----1------",
            reads:         "----1------",
        }
    }
}

#[test]
fn test_deadline_replaced_while_busy() {
    marbles! {
        actions: {
            set_deadlines: "-2--4------",
            input:         "-----------",
            busy:          "---b-------",
        },
        expectations: {
            ticks:         "0---------1",
            deadlines:     "--------1--",
            reads:         "-----------",
        }
    }
}

#[test]
fn test_ticks_while_busy() {
    marbles! {
        actions: {
            set_deadlines: "-------------------------------",
            input:         "-------------------------------",
            busy:          "-----bbbbbbbbbbbbbbbbb---------",
        },
        expectations: {
            ticks:         "0---------------------2-------3",
            deadlines:     "-------------------------------",
            reads:         "-------------------------------",
        }
    }
}
