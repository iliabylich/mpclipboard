package dev.ibylich.mpclipboard

import android.content.Context
import android.util.Log
import java.util.function.Consumer

@Suppress("unused")
class Embedded(
    private val context: Context,
    private val onReceive: Consumer<String>,
    private val onConnectivityChanged: Consumer<String>,
) {
    fun start() {
        MPClipboard.setConnectivityChangedCallback { connectivity ->
            Log.i(TAG, "connectivity: $connectivity")
            onConnectivityChanged.accept(connectivity.name)
        }
        MPClipboard.setTextReceivedCallback { text ->
            onReceive.accept(text)
        }
        val config = Store.readConfig(context)
        MPClipboard.restart(config.host, config.token, config.id)
    }

    fun sendText(text: String) {
        MPClipboard.pushText(text)
    }

    private companion object {
        const val TAG = "MPClipboardEmbedded"
    }
}
