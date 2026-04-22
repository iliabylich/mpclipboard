# MPClipboard, shared and generic part

This is a shared part of all apps that implement MPClipboard's communication protocol.

It has both Rust and C APIs.

### API

API is purely IO-driven:

1. setup an instance of `MPClipboard`
2. take a file descriptor out of it
3. throw it into **your own** event loop
4. once it's readable

### Usage in Rust

```rust
use mpclipboard_generic_client::{Config, Context, MPClipboard, Output, ConfigReadOption};

// first initialize a library (this configures a logger and TLS)
MPClipboard::init()?;

// then load a config by providing URI + token + name
let config = Config::new(uri, token, name)?;
// or by reading it from a config.toml in the $CWD
let config = Config::read(ConfigReadOption::FromLocalFile)?;
// or by reading it from $XDG_CONFIG_HOME/mpclipboard/config.toml
let config = Config::read(ConfigReadOption::FromXdgConfigDir)?;

// then initialize execution context (which internally sets up an event loop and resolves DNS)
let context = Context::new(config)?;
// NOTE: starting from here no errors can be returned

// create an instance of MPClipboard
let mut mpclipboard = MPClipboard::new(context);
// and take its file descriptor
let fd = mpclipboard.as_raw_fd()

loop {
    // FD becomes readable when there's work to do.
    // You can use literally any polling mechanism (e.g. select/poll/epoll/kqueue/io_uring/iocp)
    // to wait until FD becomes readable.
    somehow_wait_readable(fd);
    let output: Output = mpclipboard.read();

    // `output` may contain:
    // 1. received clip (either UTF-8 text or binary blob)
    // 2. information connectivity (connected/connecting/disconnected)
    println!("{:?}", output);
}
```

### Usafe in C

C API fully mirrors Rust API

```c
mpclipboard_init();

mpclipboard_Config *config = mpclipboard_config_read(MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE);
assert(config);

mpclipboard_Context *ctx = mpclipboard_context_new(config);
assert(ctx);

mpclipboard_MPClipboard *mpclipboard = mpclipboard_new(ctx);
int fd = mpclipboard_get_fd(mpclipboard);

for (;;) {
    somehow_wait_readable(fd);
    mpclipboard_Output output = mpclipboard_read(mpclipboard);

    // do something with output (tagged union)
}
```
