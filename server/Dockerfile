FROM rust:1.95-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build/clip
COPY clip/Cargo.toml clip/README.md ./
COPY clip/src ./src

WORKDIR /build/server
COPY server/Cargo.toml server/README.md ./
COPY server/src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

COPY --from=builder /build/server/target/x86_64-unknown-linux-musl/release/mpclipboard-server /usr/bin/mpclipboard-server
ENTRYPOINT ["mpclipboard-server"]
