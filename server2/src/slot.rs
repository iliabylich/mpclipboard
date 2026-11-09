use crate::{message::Message, request_reader::ID};
use core::{
    cell::{Cell, RefCell},
    future::poll_fn,
    task::{Poll, Waker},
};

#[expect(clippy::large_enum_variant)]
pub(crate) enum Slot {
    Free,
    Taken {
        id: ID,
        pending: Option<Message>,
        waker: Option<Waker>,
    },
}

pub(crate) struct SlotGuard<'a> {
    slot: &'a RefCell<Slot>,
    slots: &'a [RefCell<Slot>],
    current: &'a Cell<Option<Message>>,
}

impl<'a> SlotGuard<'a> {
    pub(crate) fn take(
        slots: &'a [RefCell<Slot>],
        current: &'a Cell<Option<Message>>,
        id: ID,
    ) -> Result<Self, ()> {
        let Some(slot) = slots
            .iter()
            .find(|slot| matches!(*slot.borrow(), Slot::Free))
        else {
            log::error!("[{id}] no free slot");
            return Err(());
        };

        *slot.borrow_mut() = Slot::Taken {
            id,
            pending: current.get(),
            waker: None,
        };
        Ok(Self {
            slot,
            slots,
            current,
        })
    }

    pub(crate) async fn notified(&self) -> Message {
        poll_fn(|cx| {
            let mut slot = self.slot.borrow_mut();
            let Slot::Taken { pending, waker, .. } = &mut *slot else {
                unreachable!("a guarded slot is always taken");
            };

            if let Some(message) = pending.take() {
                return Poll::Ready(message);
            }
            *waker = Some(cx.waker().clone());
            Poll::Pending
        })
        .await
    }

    pub(crate) fn publish(&self, message: Message) {
        if self.current.get() == Some(message) {
            return;
        }
        self.current.set(Some(message));

        let Slot::Taken { id: sender, .. } = *self.slot.borrow() else {
            unreachable!("a guarded slot is always taken");
        };
        log::info!("[{sender}] broadcasting {message:?}");

        for slot in self.slots {
            let waker = match &mut *slot.borrow_mut() {
                Slot::Taken { id, pending, waker } if *id != sender => {
                    *pending = Some(message);
                    waker.take()
                }
                Slot::Taken { .. } | Slot::Free => None,
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }
    }
}

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        *self.slot.borrow_mut() = Slot::Free;
    }
}

#[cfg(test)]
mod tests {
    use super::{Slot, SlotGuard};
    use crate::{message::tests::message, request_reader::ID};
    use core::{
        cell::{Cell, RefCell},
        future::Future,
        pin::pin,
        task::{Context, Poll},
    };
    use std::{
        string::String,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::Wake,
    };

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn id(id: &str) -> ID {
        ID::new(id).unwrap()
    }

    fn slots<const N: usize>() -> [RefCell<Slot>; N] {
        [const { RefCell::new(Slot::Free) }; N]
    }

    fn poll_notified(guard: &SlotGuard<'_>, waker: &Arc<CountingWaker>) -> Option<String> {
        let waker = waker.clone().into();
        let mut cx = Context::from_waker(&waker);
        match pin!(guard.notified()).poll(&mut cx) {
            Poll::Ready(message) => Some(String::from(message.text())),
            Poll::Pending => None,
        }
    }

    #[test]
    fn test_new_client_gets_current_message() {
        let slots = slots::<2>();
        let current = Cell::new(Some(message("hello")));
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let guard = SlotGuard::take(&slots, &current, id("a")).unwrap();
        assert_eq!(poll_notified(&guard, &waker), Some("hello".into()));
        assert_eq!(poll_notified(&guard, &waker), None);
    }

    #[test]
    fn test_new_client_without_current_message() {
        let slots = slots::<2>();
        let current = Cell::new(None);
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let guard = SlotGuard::take(&slots, &current, id("a")).unwrap();
        assert_eq!(poll_notified(&guard, &waker), None);
    }

    #[test]
    fn test_publish_skips_sender_and_wakes_others() {
        let slots = slots::<3>();
        let current = Cell::new(None);
        let a_waker = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let b_waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let a = SlotGuard::take(&slots, &current, id("a")).unwrap();
        let b = SlotGuard::take(&slots, &current, id("b")).unwrap();
        assert_eq!(poll_notified(&a, &a_waker), None);
        assert_eq!(poll_notified(&b, &b_waker), None);

        a.publish(message("from a"));

        assert_eq!(current.get(), Some(message("from a")));
        assert_eq!(a_waker.0.load(Ordering::Relaxed), 0);
        assert_eq!(b_waker.0.load(Ordering::Relaxed), 1);
        assert_eq!(poll_notified(&a, &a_waker), None);
        assert_eq!(poll_notified(&b, &b_waker), Some("from a".into()));
    }

    #[test]
    fn test_publish_same_message_is_ignored() {
        let slots = slots::<2>();
        let current = Cell::new(Some(message("same")));
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let a = SlotGuard::take(&slots, &current, id("a")).unwrap();
        let b = SlotGuard::take(&slots, &current, id("b")).unwrap();
        assert_eq!(poll_notified(&b, &waker), Some("same".into()));

        a.publish(message("same"));
        assert_eq!(poll_notified(&b, &waker), None);
        assert_eq!(waker.0.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_last_message_wins() {
        let slots = slots::<2>();
        let current = Cell::new(None);
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let a = SlotGuard::take(&slots, &current, id("a")).unwrap();
        let b = SlotGuard::take(&slots, &current, id("b")).unwrap();

        a.publish(message("one"));
        a.publish(message("two"));
        assert_eq!(poll_notified(&b, &waker), Some("two".into()));
        assert_eq!(poll_notified(&b, &waker), None);
    }

    #[test]
    fn test_drop_frees_slot() {
        let slots = slots::<1>();
        let current = Cell::new(None);

        let a = SlotGuard::take(&slots, &current, id("a")).unwrap();
        assert!(SlotGuard::take(&slots, &current, id("b")).is_err());
        drop(a);
        assert!(SlotGuard::take(&slots, &current, id("b")).is_ok());
    }
}
