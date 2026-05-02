# `MPClipboard` server

This is a central part in communication process, a server that connects clients of all flavors.

It's based on [Tokio](https://tokio.rs/) and an amazing [`tungstenite`](https://github.com/snapview/tungstenite-rs) library.

It has only one endpoint: `/` that:

+ receives incoming TCP connections
+ performs authentication (and responds with 401 if token doesn't match)
+ does handshake and switches to WebSocket protocol
+ sends the most recent clip to a newly connected client
+ starts receiving new clipboard texts from clients
+ broadcasts them to all connected clients

Authentication is based on a static token that is written in the `config.toml`.

The server itself doesn't handle any TLS, instead it expects a reverse proxy in front of it (Nginx/caddy/etc).

### Building

```
cargo build --release
```

Additionally, there's a [`debian/mpclipboard-server.service`](/debian/mpclipboard-server.service) systemd service if you need it.

### Running in Docker

We provide a Docker image on ghcr.io (GitHub container registry).

First, you need a `config.toml` file:

```toml
host = "0.0.0.0"
port = 3000
token = "s3cr3t"
```

Then:

1. optionally enable logging
2. specify port mapping
3. mount volume with a config

```sh
docker run \
    -e RUST_LOG=trace
    -p 3000:3000 \
    -v ./config.toml:/etc/mpclipboard-server/config.toml:ro \
    ghcr.io/mpclipboard/server:latest
```
