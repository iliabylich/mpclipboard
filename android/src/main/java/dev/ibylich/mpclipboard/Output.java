package dev.ibylich.mpclipboard;

import java.nio.charset.StandardCharsets;

public final class Output {
    private final Connectivity connectivity;
    private final String text;

    private Output(boolean hasConnectivity, int connectivity, byte[] text) {
        this.connectivity = hasConnectivity ? MPClipboard.connectivityFrom(connectivity) : null;
        this.text = text == null ? null : new String(text, StandardCharsets.UTF_8);
    }

    public Connectivity getConnectivity() {
        return connectivity;
    }

    public String getText() {
        return text;
    }
}
