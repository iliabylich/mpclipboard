package dev.ibylich.mpclipboard;

import dev.ibylich.mpclipboard.IMPClipboardCallback;

oneway interface IMPClipboardService {
    void onNewLocalText(String text);
    void registerRemoteTextCallback(IMPClipboardCallback callback);
    void unregisterRemoteTextCallback(IMPClipboardCallback callback);
}
