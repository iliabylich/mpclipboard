use crate::Buffer;
use anyhow::{Context, Result};
use core::num::NonZeroUsize;

pub(crate) fn as_chunks_with_guaranteed_trailer<const BUFSIZE: usize>(
    buf: &[u8],
) -> (impl Iterator<Item = Buffer<BUFSIZE>>, Buffer<BUFSIZE>) {
    const CHUNK_SIZE: usize = 20;

    let (head, tail) = buf.split_at(
        buf.len()
            .checked_sub(CHUNK_SIZE)
            .unwrap_or_else(|| unreachable!("bug")),
    );

    let chunks = head.chunks(CHUNK_SIZE).map(|chunk| {
        Buffer::from_slice(chunk).unwrap_or_else(|| unreachable!("chunk fits into a buffer"))
    });
    let trailer =
        Buffer::from_slice(tail).unwrap_or_else(|| unreachable!("trailer fits into a buffer"));

    (chunks, trailer)
}

pub(crate) fn buffer<const N: usize>(bytes: &[u8]) -> Result<Buffer<N>> {
    Buffer::from_slice(bytes).context("bytes don't fit into a buffer")
}

pub(crate) fn non_zero_usize(n: usize) -> Result<NonZeroUsize> {
    NonZeroUsize::new(n).context("must be non-zero")
}
