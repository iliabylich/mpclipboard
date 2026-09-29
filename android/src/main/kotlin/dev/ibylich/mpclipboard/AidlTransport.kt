package dev.ibylich.mpclipboard

import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.RemoteCallbackList
import android.os.RemoteException

object AidlTransport {
    private val mainHandler = Handler(Looper.getMainLooper())

    @Volatile
    private var callbacks = RemoteCallbackList<IMPClipboardCallback>()
    private var onNewTextCallback: ((String) -> Unit)? = null

    internal val binder: IBinder = object : IMPClipboardService.Stub() {
        override fun onNewLocalText(text: String) {
            mainHandler.post {
                onNewTextCallback?.invoke(text)
            }
        }

        override fun registerRemoteTextCallback(callback: IMPClipboardCallback) {
            callbacks.register(callback)
        }

        override fun unregisterRemoteTextCallback(callback: IMPClipboardCallback) {
            callbacks.unregister(callback)
        }
    }

    fun setOnNewTextCallback(callback: (String) -> Unit) {
        onNewTextCallback = callback
    }

    fun pushText(text: String) {
        val count = callbacks.beginBroadcast()
        try {
            for (index in 0 until count) {
                try {
                    callbacks.getBroadcastItem(index).onNewRemoteText(text)
                } catch (_: RemoteException) { }
            }
        } finally {
            callbacks.finishBroadcast()
        }
    }

    internal fun onServiceDestroyed() {
        callbacks.kill()
        callbacks = RemoteCallbackList()
    }
}
