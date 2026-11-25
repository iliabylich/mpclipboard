## The server

The server is single-threaded and it can handle up to 10 clients at once concurrently (can be changed, see `constants.rs`).

Configuration:

```sh
MPCLIPBOARD_BIND_ADDRESS=127.0.0.1:3000   # where the reverse proxy connects to, IPv4 or IPv6
MPCLIPBOARD_TOKEN=s3cr3t                  # shared with all clients, up to 20 bytes
RUST_LOG=info                             # error|info|trace, info is the default
```

The `.deb` package installs a systemd service that reads these variables from `/etc/mpclipboard-server/env`.
