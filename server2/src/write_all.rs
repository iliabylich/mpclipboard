use crate::{ServerContext, errors::ClientError, id::ID};
use externally_driven_async_rt::net::BorrowedWriteHalf;

pub(crate) async fn write_all(
    ctx: &ServerContext<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    mut buf: &[u8],
    id: ID,
) -> Result<(), ClientError> {
    while !buf.is_empty() {
        match tx.write(ctx, buf).await {
            Ok(0) => return Err(ClientError::WriteZero { id }),
            Ok(len) => buf = &buf[len..],
            Err(err) => return Err(ClientError::Write { id, err }),
        }
    }
    Ok(())
}
