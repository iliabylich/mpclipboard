package dev.ibylich.mpclipboard

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.os.RemoteCallbackList
import android.os.RemoteException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class MPClipboardService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val callbacks = RemoteCallbackList<IMPClipboardCallback>()
    private lateinit var connectionManager: MPClipboardConnectionManager

    private val binder = object : IMPClipboardService.Stub() {
        override fun onNewLocalText(text: String) {
            scope.launch {
                connectionManager.pushText(text)
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

    override fun onCreate() {
        super.onCreate()
        connectionManager = MPClipboardConnectionManager(this)
        connectionManager.start()
        scope.launch {
            connectionManager.incomingText.collect(::notifyRemoteTextCallbacks)
        }
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onDestroy() {
        callbacks.kill()
        connectionManager.close()
        scope.cancel()
        super.onDestroy()
    }

    private fun notifyRemoteTextCallbacks(text: String) {
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
