use crate::tls_stream::{TlsError, state::TlsState, step::Step};
use core::{cell::RefCell, task::Poll};
use externally_driven_async_rt::{Context, State, io::AsyncWrite, net::BorrowedWriteHalf};
use rustix::io::Errno;

pub(crate) struct TlsWriteHalf<'s> {
    tx: BorrowedWriteHalf<'s>,
    state: &'s RefCell<TlsState>,
}

impl<'s> TlsWriteHalf<'s> {
    pub(super) const fn new(tx: BorrowedWriteHalf<'s>, state: &'s RefCell<TlsState>) -> Self {
        Self { tx, state }
    }

    pub(super) async fn transmit<
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &self,
        ctx: &Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
    ) -> Result<(), TlsError> {
        while self.try_transmit()?.is_pending() {
            self.tx.writable(ctx).await;
        }
        Ok(())
    }

    fn try_transmit(&self) -> Result<Poll<()>, TlsError> {
        let mut state = self.state.borrow_mut();
        loop {
            let pending = state.outgoing.pending();
            if pending.is_empty() {
                return Ok(Poll::Ready(()));
            }
            match self.tx.try_write(pending) {
                Ok(0) => return Err(TlsError::WriteZero),
                Ok(len) => state.outgoing.sent(len),
                Err(Errno::AGAIN) => return Ok(Poll::Pending),
                Err(errno) => return Err(TlsError::Write(errno)),
            }
        }
    }

    fn try_encrypt(&self, buf: &[u8]) -> Result<Poll<usize>, TlsError> {
        let mut state = self.state.borrow_mut();
        loop {
            match state.step()? {
                Step::Writable => return Ok(Poll::Ready(state.encrypt(buf)?)),
                Step::Transmit => return Ok(Poll::Pending),
                Step::Plaintext => {}
                Step::PeerClosed => state.peer_closed = true,
                Step::Blocked => return Err(TlsError::HandshakeNotComplete),
            }
        }
    }
}

impl AsyncWrite for TlsWriteHalf<'_> {
    type Error = TlsError;

    async fn write<
        'a,
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &'a mut self,
        ctx: &'a Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
        buf: &'a [u8],
    ) -> Result<usize, TlsError> {
        loop {
            self.transmit(ctx).await?;
            if let Poll::Ready(len) = self.try_encrypt(buf)? {
                return Ok(len);
            }
        }
    }

    async fn flush<
        'a,
        S: State,
        const TASKS: usize,
        const TASK_SIZE: usize,
        const ROOT_TASK_SIZE: usize,
        const TIMERS: usize,
        const FDS: usize,
    >(
        &'a mut self,
        ctx: &'a Context<'_, S, TASKS, TASK_SIZE, ROOT_TASK_SIZE, TIMERS, FDS>,
    ) -> Result<(), TlsError> {
        self.transmit(ctx).await
    }
}
