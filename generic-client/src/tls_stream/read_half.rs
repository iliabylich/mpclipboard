use crate::{
    runtime::BorrowedReadHalf,
    tls_stream::{TlsError, state::TlsState, step::Step},
};
use core::{cell::RefCell, task::Poll};
use externally_driven_async_rt::io::AsyncRead;
use rustix::io::Errno;

pub(crate) struct TlsReadHalf<'s> {
    rx: BorrowedReadHalf<'s>,
    state: &'s RefCell<TlsState>,
}

impl<'s> TlsReadHalf<'s> {
    pub(super) const fn new(rx: BorrowedReadHalf<'s>, state: &'s RefCell<TlsState>) -> Self {
        Self { rx, state }
    }

    pub(super) async fn fill(&mut self) -> Result<(), TlsError> {
        while self.try_fill()?.is_pending() {
            self.rx.readable().await;
        }
        Ok(())
    }

    fn try_fill(&self) -> Result<Poll<()>, TlsError> {
        let mut state = self.state.borrow_mut();
        match self.rx.try_read(state.incoming.spare_mut()?) {
            Ok(0) => Err(TlsError::Closed),
            Ok(len) => {
                state.incoming.filled(len);
                Ok(Poll::Ready(()))
            }
            Err(Errno::AGAIN) => Ok(Poll::Pending),
            Err(errno) => Err(TlsError::Read(errno)),
        }
    }

    fn try_read(&self, buf: &mut [u8]) -> Result<Poll<usize>, TlsError> {
        let mut state = self.state.borrow_mut();
        loop {
            if !state.plaintext.is_empty() {
                return Ok(Poll::Ready(state.plaintext.take(buf)));
            }
            if state.peer_closed {
                return Ok(Poll::Ready(0));
            }
            match state.step()? {
                Step::Plaintext => {}
                Step::PeerClosed => state.peer_closed = true,
                Step::Transmit | Step::Blocked | Step::Writable => return Ok(Poll::Pending),
            }
        }
    }
}

impl AsyncRead for TlsReadHalf<'_> {
    type Error = TlsError;

    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize, TlsError> {
        loop {
            if let Poll::Ready(len) = self.try_read(buf)? {
                return Ok(len);
            }
            self.fill().await?;
        }
    }
}
