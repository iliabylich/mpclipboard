use core::num::NonZeroUsize;

pub(crate) fn non_zero_usize(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
