run:
    RUST_LOG=debug cargo run

now := `date +%s`

build-deb-package:
    cargo build --target=x86_64-unknown-linux-musl --release

    strip target/x86_64-unknown-linux-musl/release/mpclipboard-server
    cargo deb --deb-revision="{{now}}" -p mpclipboard-server --target=x86_64-unknown-linux-musl

container-build version:
    podman build -t ghcr.io/mpclipboard/server:{{version}} .

container-push version:
    podman push ghcr.io/mpclipboard/server:{{version}}

container-mark-latest version:
    podman tag ghcr.io/mpclipboard/server:{{version}} ghcr.io/mpclipboard/server:latest
    podman push ghcr.io/mpclipboard/server:latest
