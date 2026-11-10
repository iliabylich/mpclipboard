use crate::{constants::MAX_CLIENTS, id::ID, message::Message};
use core::{
    cell::{Cell, RefCell},
    future::poll_fn,
    task::{Poll, Waker},
};

pub(crate) struct PubSub {
    current: Cell<Option<Message>>,
    slots: [RefCell<Slot>; MAX_CLIENTS],
}

#[expect(clippy::large_enum_variant)]
enum Slot {
    Free,
    Taken {
        pending: Option<Message>,
        waker: Option<Waker>,
    },
}

impl PubSub {
    pub(crate) fn new() -> Self {
        Self {
            current: Cell::new(None),
            slots: [const { RefCell::new(Slot::Free) }; MAX_CLIENTS],
        }
    }

    pub(crate) fn subscribe(&self) -> Option<PubSubHandle<'_>> {
        let slot = self
            .slots
            .iter()
            .find(|slot| matches!(*slot.borrow(), Slot::Free))?;

        *slot.borrow_mut() = Slot::Taken {
            pending: self.current.get(),
            waker: None,
        };
        Some(PubSubHandle { pubsub: self, slot })
    }
}

pub(crate) struct PubSubHandle<'a> {
    pubsub: &'a PubSub,
    slot: &'a RefCell<Slot>,
}

impl PubSubHandle<'_> {
    pub(crate) async fn receive(&self) -> Message {
        poll_fn(|cx| {
            let mut slot = self.slot.borrow_mut();
            let Slot::Taken { pending, waker } = &mut *slot else {
                unreachable!("a handle's slot is always taken");
            };

            if let Some(message) = pending.take() {
                return Poll::Ready(message);
            }
            *waker = Some(cx.waker().clone());
            Poll::Pending
        })
        .await
    }

    pub(crate) fn publish(&self, id: ID, message: Message) {
        let current = &self.pubsub.current;
        if current.get() == Some(message) {
            return;
        }
        current.set(Some(message));

        info!("[{id}] publishing {message:?}");

        for slot in &self.pubsub.slots {
            if core::ptr::eq(slot, self.slot) {
                continue;
            }
            let waker = match &mut *slot.borrow_mut() {
                Slot::Taken { pending, waker } => {
                    *pending = Some(message);
                    waker.take()
                }
                Slot::Free => None,
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }
    }
}

impl Drop for PubSubHandle<'_> {
    fn drop(&mut self) {
        *self.slot.borrow_mut() = Slot::Free;
    }
}

#[cfg(test)]
mod tests {
    use super::{PubSub, PubSubHandle};
    use crate::{constants::MAX_CLIENTS, id::ID, message::tests::message};
    use core::{
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
        vec::Vec,
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

    fn poll_receive(pubsub: &PubSubHandle<'_>, waker: &Arc<CountingWaker>) -> Option<String> {
        let waker = waker.clone().into();
        let mut cx = Context::from_waker(&waker);
        match pin!(pubsub.receive()).poll(&mut cx) {
            Poll::Ready(message) => Some(String::from(message.text())),
            Poll::Pending => None,
        }
    }

    #[test]
    fn test_new_client_gets_current_message() {
        let pubsub = PubSub::new();
        pubsub.current.set(Some(message("hello")));
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let handle = pubsub.subscribe().unwrap();
        assert_eq!(poll_receive(&handle, &waker), Some("hello".into()));
        assert_eq!(poll_receive(&handle, &waker), None);
    }

    #[test]
    fn test_new_client_without_current_message() {
        let pubsub = PubSub::new();
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let handle = pubsub.subscribe().unwrap();
        assert_eq!(poll_receive(&handle, &waker), None);
    }

    #[test]
    fn test_publish_skips_sender_and_wakes_others() {
        let pubsub = PubSub::new();
        let a_waker = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let b_waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let a = pubsub.subscribe().unwrap();
        let b = pubsub.subscribe().unwrap();
        assert_eq!(poll_receive(&a, &a_waker), None);
        assert_eq!(poll_receive(&b, &b_waker), None);

        a.publish(id("a"), message("from a"));

        assert_eq!(pubsub.current.get(), Some(message("from a")));
        assert_eq!(a_waker.0.load(Ordering::Relaxed), 0);
        assert_eq!(b_waker.0.load(Ordering::Relaxed), 1);
        assert_eq!(poll_receive(&a, &a_waker), None);
        assert_eq!(poll_receive(&b, &b_waker), Some("from a".into()));
    }

    #[test]
    fn test_publish_reaches_other_connection_with_same_id() {
        let pubsub = PubSub::new();
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let old = pubsub.subscribe().unwrap();
        let new = pubsub.subscribe().unwrap();
        assert_eq!(poll_receive(&old, &waker), None);

        new.publish(id("a"), message("from new"));
        assert_eq!(poll_receive(&old, &waker), Some("from new".into()));
    }

    #[test]
    fn test_publish_same_message_is_ignored() {
        let pubsub = PubSub::new();
        pubsub.current.set(Some(message("same")));
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let a = pubsub.subscribe().unwrap();
        let b = pubsub.subscribe().unwrap();
        assert_eq!(poll_receive(&b, &waker), Some("same".into()));

        a.publish(id("a"), message("same"));
        assert_eq!(poll_receive(&b, &waker), None);
        assert_eq!(waker.0.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_last_message_wins() {
        let pubsub = PubSub::new();
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));

        let a = pubsub.subscribe().unwrap();
        let b = pubsub.subscribe().unwrap();

        a.publish(id("a"), message("one"));
        a.publish(id("a"), message("two"));
        assert_eq!(poll_receive(&b, &waker), Some("two".into()));
        assert_eq!(poll_receive(&b, &waker), None);
    }

    #[test]
    fn test_drop_frees_slot() {
        let pubsub = PubSub::new();

        let mut handles: Vec<_> = (0..MAX_CLIENTS)
            .map(|_| pubsub.subscribe().unwrap())
            .collect();
        assert!(pubsub.subscribe().is_none());
        handles.pop();
        assert!(pubsub.subscribe().is_some());
    }
}
