use crate::Buffer;
use core::num::NonZeroUsize;

pub(crate) fn as_chunks_with_guaranteed_trailer<const BUFSIZE: usize>(
    buf: &[u8],
) -> (impl Iterator<Item = Buffer<BUFSIZE>>, Buffer<BUFSIZE>) {
    const CHUNK_SIZE: usize = 20;

    let (head, tail) = buf.split_at(buf.len() - CHUNK_SIZE);

    let chunks = head.chunks(CHUNK_SIZE).map(buffer);
    let trailer = buffer(tail);

    (chunks, trailer)
}

pub(crate) fn buffer<const N: usize>(bytes: &[u8]) -> Buffer<N> {
    Buffer::from_slice(bytes).unwrap()
}

pub(crate) fn non_zero_usize(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
