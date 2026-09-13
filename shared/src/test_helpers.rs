use core::num::NonZeroUsize;

type Chunk<const BUFSIZE: usize> = ([u8; BUFSIZE], NonZeroUsize);

pub(crate) fn as_chunks_with_guaranteed_trailer<const BUFSIZE: usize>(
    buf: &[u8],
) -> (impl Iterator<Item = Chunk<BUFSIZE>>, Chunk<BUFSIZE>) {
    let (head, tail) = buf.split_at(buf.len() - 20);

    let chunks = head.chunks(20).filter_map(|chunk| {
        let len = NonZeroUsize::new(chunk.len())?;
        let mut buf = [0; BUFSIZE];
        buf[..chunk.len()].copy_from_slice(chunk);
        Some((buf, len))
    });

    let mut trailer = [0; BUFSIZE];
    trailer[..tail.len()].copy_from_slice(tail);
    let trailer = (trailer, NonZeroUsize::new(tail.len()).unwrap());

    (chunks, trailer)
}
