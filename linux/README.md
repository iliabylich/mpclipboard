# Linux client

A simple client that:

1. implements communication over WebSocket using [`generic-client`](https://github.com/mpclipboard/generic-client)
2. integrates with Wayland clipboard to read/write clipboard text
3. shows a tray icon with 5 last clips (implements `org.kde.StatusNotifierItem` spec)
