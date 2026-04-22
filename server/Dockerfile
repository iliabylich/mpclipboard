FROM rust:1.90-alpine AS builder

RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock README.md ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mpclipboard-server /usr/bin/mpclipboard-server
ENTRYPOINT ["mpclipboard-server"]
