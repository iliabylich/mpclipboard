package dev.ibylich.mpclipboard;

import java.nio.charset.StandardCharsets;

public final class MPClipboard implements AutoCloseable {
    static {
        System.loadLibrary("mpclipboard_android");
    }

    private static final int CONNECTING = connectivityConnecting();
    private static final int CONNECTED = connectivityConnected();
    private static final int DISCONNECTED = connectivityDisconnected();

    private long handle;

    private MPClipboard(long handle) {
        this.handle = handle;
    }

    public static MPClipboard newInline(String url, String token, String id) {
        long handle = mpclipboard_new_inline(
                url.getBytes(StandardCharsets.UTF_8),
                token.getBytes(StandardCharsets.UTF_8),
                id.getBytes(StandardCharsets.UTF_8));
        if (handle == 0) {
            return null;
        }
        return new MPClipboard(handle);
    }

    public int getFd() {
        return mpclipboard_get_fd(handle);
    }

    public Output read() {
        return mpclipboard_read(handle);
    }

    public void pushText(String text) {
        mpclipboard_push_text(handle, text.getBytes(StandardCharsets.UTF_8));
    }

    @Override
    public void close() {
        mpclipboard_drop(handle);
        handle = 0;
    }

    static Connectivity connectivityFrom(int tag) {
        if (tag == CONNECTING) {
            return Connectivity.CONNECTING;
        }
        if (tag == CONNECTED) {
            return Connectivity.CONNECTED;
        }
        if (tag == DISCONNECTED) {
            return Connectivity.DISCONNECTED;
        }
        throw new IllegalStateException("unknown native connectivity tag: " + tag);
    }

    private static native int connectivityConnecting();

    private static native int connectivityConnected();

    private static native int connectivityDisconnected();

    private static native long mpclipboard_new_inline(byte[] url, byte[] token, byte[] id);

    private static native int mpclipboard_get_fd(long handle);

    private static native Output mpclipboard_read(long handle);

    private static native void mpclipboard_push_text(long handle, byte[] text);

    private static native void mpclipboard_drop(long handle);
}
