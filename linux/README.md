# Linux client

A simple client that:

1. talks to the server using [`generic-client`](/generic-client)
2. integrates with Wayland clipboard to read/write clipboard text (via the `ext-data-control` protocol, your compositor must support it)
3. shows a tray icon (implements `org.kde.StatusNotifierItem` spec):
    + the color of the icon shows connectivity: yellow - connecting, green - connected, red - disconnected
    + the menu shows 5 last clips, `S` for sent, `R` for received

### Configuration

If you have your server up and running, make sure to create a config in `~/.config/mpclipboard/config.toml`:

```toml
url = "https://host:443" # must be https:// with an explicit port
token = "s3cr3t"
id = "linux-client"
```

Each config value can be overridden with `MPCLIPBOARD_URL`, `MPCLIPBOARD_TOKEN` and `MPCLIPBOARD_ID` environment variables.

### Building

```sh
mise dev                                                    # builds and runs a debug client against a local server
mise release:deb --version <version> --out <absolute path>  # builds a release client and packs it into a .deb
```

The `.deb` package installs a systemd user service:

```sh
systemctl --user enable --now mpclipboard-client
```
