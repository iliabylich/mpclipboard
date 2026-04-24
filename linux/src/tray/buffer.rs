pub(crate) struct Buffer<const N: usize, T> {
    buf: Vec<T>,
}

impl<const N: usize, T> Buffer<N, T> {
    pub(crate) fn new() -> Self {
        Self { buf: vec![] }
    }

    pub(crate) fn push(&mut self, item: T) {
        self.buf.push(item);
        if self.buf.len() > N {
            self.buf.remove(0);
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.buf.iter()
    }
}
