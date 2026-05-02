use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone)]
pub(crate) struct Timer {
    inner: Rc<Cell<u64>>,
}

impl Timer {
    pub(crate) fn new() -> Self {
        Self {
            inner: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn now(&self) -> u64 {
        self.inner.get()
    }

    pub(crate) fn tick(&self) {
        let value = self.inner.get().wrapping_add(1);
        log::trace!("tick {value}");
        self.inner.set(value);
    }
}
