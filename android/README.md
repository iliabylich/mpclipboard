## Android library

A Java wrapper around the native [`generic-client`](/generic-client), packaged as an `.aar` (`classes.jar` + `jni/arm64-v8a/libmpclipboard_android.so`).

What it does NOT provide or specify:

1. UI
2. manifest components
3. permissions
4. clipboard integration

The host app builds all of that itself, the same way the [Linux](/linux) and [macOS](/macos) clients do.

### Building

```sh
mise build:debug                      # build/outputs/aar/mpclipboard-debug.aar
mise build:release                    # build/outputs/aar/mpclipboard-release.aar
mise release --out <absolute path>    # build:release + copy the .aar to <absolute path>
```

### Integration

1. Put the `.aar` into your app (e.g. `libs/mpclipboard.aar`) and add `implementation(files("libs/mpclipboard.aar"))`.
2. Declare `<uses-permission android:name="android.permission.INTERNET" />` in your app's manifest; the library doesn't declare it for you.

ProGuard/R8 rules for the JNI side ship inside the `.aar`.

### API

```java
package dev.ibylich.mpclipboard;

public final class MPClipboard implements AutoCloseable {
    public static MPClipboard newInline(String url, String token, String id); // null on failure
    public int getFd();
    public Output read();              // null if there's nothing to report
    public void pushText(String text);
    public void close();
}

public final class Output {
    public Connectivity getConnectivity(); // null if unchanged
    public String getText();               // null if no new text
}

public enum Connectivity { CONNECTING, CONNECTED, DISCONNECTED }
```

All calls must happen on one thread. Calling anything after `close()` aborts the process.

### Example: main `Looper`

```java
MPClipboard client = MPClipboard.newInline(url, token, id);
ParcelFileDescriptor pfd = ParcelFileDescriptor.fromFd(client.getFd());
MessageQueue queue = Looper.getMainLooper().getQueue();

queue.addOnFileDescriptorEventListener(
        pfd.getFileDescriptor(),
        MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT,
        (fd, events) -> {
            Output output = client.read();
            if (output != null) {
                if (output.getConnectivity() != null) { /* update UI */ }
                if (output.getText() != null) { /* put into clipboard */ }
            }
            return MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT;
        });

client.pushText("copied text");

queue.removeOnFileDescriptorEventListener(pfd.getFileDescriptor());
pfd.close();
client.close();
```

`ParcelFileDescriptor.fromFd` duplicates the fd, so the copy is yours to close; the original stays owned by `MPClipboard`.
