## generic-client

All clients built on top of the generic client (except the Android library, which takes the values inline) read `$XDG_CONFIG_HOME/mpclipboard/config.toml` or `~/.config/mpclipboard/config.toml`:

```toml
url = "https://your.domain.com:443" # must be https:// with an explicit port
token = "s3cr3t"                    # must match the server's MPCLIPBOARD_TOKEN, up to 20 bytes
id = "my-laptop"                    # any name of this device, up to 20 bytes, shows up in server logs
```

Each value can be overridden with `MPCLIPBOARD_URL`, `MPCLIPBOARD_TOKEN` and `MPCLIPBOARD_ID` environment variables.

You can take a look at [bindings.h](/generic-client-ffi/bindings.h).

### API

The generic client exposes a single file descriptor that you put into your own event loop (`poll`/`epoll`/`kqueue`/`Looper`/`DispatchSource`/etc.). Every time it's readable you call `read()` which returns an optional connectivity change and an optional new text. Copied text goes in via `push_text()`. Only one instance can be created per process.

There are demo REPL clients (stdin lines are sent to the server, everything received from the server is printed to stdout):

+ [`cli`](/generic-client/examples/cli.rs) - Rust, uses `poll`, can run on Linux/macOS
+ [`poll-cli`](/poll-cli) - C, uses the C bindings and `poll`, can run on Linux/macOS
