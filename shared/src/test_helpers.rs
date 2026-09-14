use anyhow::{Context, Result};
use core::num::NonZeroUsize;

type Chunk<const BUFSIZE: usize> = ([u8; BUFSIZE], NonZeroUsize);

pub(crate) fn as_chunks_with_guaranteed_trailer<const BUFSIZE: usize>(
    buf: &[u8],
) -> (impl Iterator<Item = Chunk<BUFSIZE>>, Chunk<BUFSIZE>) {
    const CHUNK_SIZE: usize = 20;

    let (head, tail) = buf.split_at(buf.len() - CHUNK_SIZE);

    let chunks = head.chunks(CHUNK_SIZE).filter_map(|chunk| {
        let len = NonZeroUsize::new(chunk.len())?;
        let mut buf = [0; BUFSIZE];
        buf[..chunk.len()].copy_from_slice(chunk);
        Some((buf, len))
    });

    let mut trailer = [0; BUFSIZE];
    trailer[..tail.len()].copy_from_slice(tail);
    let trailer = (
        trailer,
        NonZeroUsize::new(CHUNK_SIZE).expect("constant > 0"),
    );

    (chunks, trailer)
}

pub(crate) fn non_zero_usize(n: usize) -> Result<NonZeroUsize> {
    NonZeroUsize::new(n).context("must be non-zero")
}
