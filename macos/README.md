# macOS client

A simple client that:

1. implements communication over WebSocket using `generic-client` and FFI
2. integrates with macOS clipboard to read/write clipboard text
3. shows a tray icon with last 5 clips
4. shows a system notification every time a new text is received

### Configuration

If you have your server up and running, make sure to create a config in `~/.config/mpclipboard/config.toml`:

```toml
uri = "ws://host:port" # or "wss://host:port" if you use SSL
token = "s3cr3t"
name = "macos-client"
```

### Building

```sh
just build Release # or Debug
```

The app will be in `build/Release/mpclipboard.app` (or `build/Debug/mpclipboard.app`).

### Other notes

1. Debug build of the app always reads local config file from `config.toml`, Release build always reads from `$HOME/.config/mpclipboard/config.toml`.
2. The app is not signed, so you need to manually approve running it locally (or take the binary manually out of the quarantine).
3. There's no API in macOS to subscribe to clipboard changes, so we poll it manually every 0.1s. If you know a better way to track clipboard updates please open an issue.

### Troubleshooting

If you have installed the app, it should be located at `/Applications/mpclipboard.app`. macOS apps are just directories with a pre-defined, well-known structure. The main executable file is located at `/Applications/mpclipboard.app/Contents/MacOS/mpclipboard`.

If the app doesn't start, first try running it from the console:

```sh
RUST_LOG=info /Applications/mpclipboard.app/Contents/MacOS/mpclipboard
```

This should most probably give you some insight about what's missing or incorrect (most probably it's the config). If the error doesn't explain what's wrong, feel free to create an issue.
