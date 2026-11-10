use crate::ServerContext;
use externally_driven_async_rt::net::BorrowedWriteHalf;

pub(crate) async fn write_all(
    ctx: &ServerContext<'_>,
    tx: &mut BorrowedWriteHalf<'_>,
    mut buf: &[u8],
) -> Result<(), ()> {
    while !buf.is_empty() {
        match tx.write(ctx, buf).await {
            Ok(0) => {
                log::error!("write() wrote 0 bytes");
                return Err(());
            }
            Ok(len) => buf = &buf[len..],
            Err(err) => {
                log::error!("write() failed: {err}");
                return Err(());
            }
        }
    }
    Ok(())
}
