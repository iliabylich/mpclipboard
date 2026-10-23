# Vendored `polling`

A trimmed copy of [`polling` 3.11.0](https://github.com/smol-rs/polling)
(MIT OR Apache-2.0, see `LICENSE-MIT` / `LICENSE-APACHE`).

The file layout mirrors upstream so that upgrading is a matter of diffing
upstream's `src/` against the previous version and replaying the changes here:

| here            | upstream           |
|-----------------|--------------------|
| `mod.rs`        | `src/lib.rs`       |
| `epoll.rs`      | `src/epoll.rs`     |
| `kqueue.rs`     | `src/kqueue.rs`    |
| `os.rs`         | `src/os.rs`        |
| `os/kqueue.rs`  | `src/os/kqueue.rs` |

## Changes compared to upstream

All of them follow from the fact that `EventLoopWithTimer` is single-threaded
and never wakes the poller up from another thread:

1. **No notifier** (`Poller::notify`, `NOTIFY_KEY`, eventfd/pipe on Linux,
   `EVFILT_USER` on macOS). Upstream clears and re-arms the eventfd after
   every `epoll_wait`, which costs 2 syscalls per `wait()`.
2. **No internal timerfd on epoll.** Upstream re-arms it before every
   `epoll_wait` to get sub-millisecond timeouts, which costs 2 syscalls per
   `wait()`. The timeout is passed to `epoll_wait` directly instead (this is
   upstream's own Redox code path).
3. **No `Mutex`/`AtomicBool` in `Poller`** (only needed for concurrent
   `wait()`/`notify()`).
4. Only the epoll (Linux, Android) and kqueue (macOS) backends are kept; no
   `tracing`, no `Signal`/`Process` kqueue filters, no API we don't call.
