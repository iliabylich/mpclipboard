package dev.ibylich.mpclipboard;

/** Receives clipboard text which arrived from another MPClipboard client. */
oneway interface IMPClipboardCallback {
    void onNewRemoteText(String text);
}
