# `MPClipboard`, shared and generic part

This is a shared part of all apps that implement `MPClipboard`'s communication protocol.

It has both Rust and C APIs.

### API

API is purely IO-driven:

1. setup an instance of `MPClipboard`
2. take a file descriptor out of it
3. throw it into **your own** event loop
4. wait until it becomes readable
5. read, go to 4

### Usage in Rust

```rust,ignore
use mpclipboard_generic_client::{MPClipboard, Output};

// create an instance of MPClipboard
let mut mpclipboard = MPClipboard::new_with_xdg_config()?;
// or MPClipboard::new_with_local_config()
// or MPClipboard::new_inline("https://your.host:443", "<TOKEN>", "<ID>")

// and take its file descriptor
let fd = mpclipboard.as_raw_fd();

loop {
    // FD becomes readable when there's work to do.
    // You can use literally any polling mechanism (e.g. select/poll/epoll/kqueue/io_uring/iocp)
    // to wait until FD becomes readable.
    somehow_wait_readable(fd);
    let output: Output = mpclipboard.read()?;

    // `output` may contain:
    // 1. received UTF-8 text
    // 2. information about connectivity (connected/connecting/disconnected)
    // 3. both
    // 4. none of them if it managed to read only a part of the text
    println!("{:?}", output);
}
```

### Usage in C

C API fully mirrors Rust API

```c
mpclipboard_MPClipboard *mpclipboard = mpclipboard_new_with_xdg_config();
// or mpclipboard_new_with_local_config()
// or mpclipboard_new_inline(url, token, id)
assert(mpclipboard);

int fd = mpclipboard_get_fd(mpclipboard);

for (;;) {
    somehow_wait_readable(fd);
    mpclipboard_Output output = mpclipboard_read(mpclipboard);

    // do something with output (struct)
}
```
