### poll example

This is a demo of a C client that uses [C bindings](/generic-client/bindings.h) of `generic-client` and `poll()` as a polling mechanism.

It reads lines from stdin and sends them to the server, at the same time it prints every connectivity change and every text received from the server.

```sh
mise dev # builds generic-client, compiles poll-cli.c and runs it against a local server
```
