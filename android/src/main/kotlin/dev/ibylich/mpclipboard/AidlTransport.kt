package dev.ibylich.mpclipboard

import android.os.IBinder
import android.os.RemoteCallbackList
import android.os.RemoteException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

object AidlTransport {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var callbacks = RemoteCallbackList<IMPClipboardCallback>()
    private var onNewTextCallback: ((String) -> Unit)? = null

    internal val binder: IBinder = object : IMPClipboardService.Stub() {
        override fun onNewLocalText(text: String) {
            scope.launch {
                onNewTextCallback?.invoke(text)
            }
        }

        override fun registerRemoteTextCallback(callback: IMPClipboardCallback) {
            scope.launch {
                callbacks.register(callback)
            }
        }

        override fun unregisterRemoteTextCallback(callback: IMPClipboardCallback) {
            scope.launch {
                callbacks.unregister(callback)
            }
        }
    }

    fun setOnNewTextCallback(callback: (String) -> Unit) {
        onNewTextCallback = callback
    }

    fun pushText(text: String) {
        scope.launch {
            val count = callbacks.beginBroadcast()
            try {
                for (index in 0 until count) {
                    try {
                        callbacks.getBroadcastItem(index).onNewRemoteText(text)
                    } catch (_: RemoteException) {
                        // RemoteCallbackList removes dead callback binders automatically.
                    }
                }
            } finally {
                callbacks.finishBroadcast()
            }
        }
    }

    internal fun onServiceDestroyed() {
        callbacks.kill()
        callbacks = RemoteCallbackList()
    }
}
