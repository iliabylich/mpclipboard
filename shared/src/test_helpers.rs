use crate::Buffer;
use core::num::NonZeroUsize;
use generic_array::ArrayLength;

pub(crate) fn as_chunks_with_guaranteed_trailer<N: ArrayLength>(
    buf: &[u8],
) -> (impl Iterator<Item = Buffer<N>>, Buffer<N>) {
    const CHUNK_SIZE: usize = 20;

    let (head, tail) = buf.split_at(buf.len() - CHUNK_SIZE);

    let chunks = head.chunks(CHUNK_SIZE).map(buffer);
    let trailer = buffer(tail);

    (chunks, trailer)
}

pub(crate) fn buffer<N: ArrayLength>(bytes: &[u8]) -> Buffer<N> {
    Buffer::from_slice(bytes).unwrap()
}

pub(crate) fn non_zero_usize(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
