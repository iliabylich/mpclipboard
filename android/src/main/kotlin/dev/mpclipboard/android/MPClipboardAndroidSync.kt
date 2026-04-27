package dev.mpclipboard.android

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class MPClipboardAndroidSync(
    context: Context,
    private val connectionManager: MPClipboardConnectionManager = MPClipboardConnectionManager(context),
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate),
) {
    private val appContext = context.applicationContext
    private val clipboardManager =
        appContext.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    private var incomingJob: Job? = null
    private var suppressNextClipboardText: String? = null
    private var isStarted = false

    private val clipboardListener = ClipboardManager.OnPrimaryClipChangedListener {
        val text = clipboardText() ?: return@OnPrimaryClipChangedListener
        if (text == suppressNextClipboardText) {
            suppressNextClipboardText = null
            return@OnPrimaryClipChangedListener
        }
        connectionManager.pushText(text)
    }

    fun start() {
        if (isStarted) {
            return
        }
        isStarted = true
        connectionManager.start()
        clipboardManager.addPrimaryClipChangedListener(clipboardListener)
        if (incomingJob?.isActive == true) {
            return
        }
        incomingJob = scope.launch {
            connectionManager.incomingText.collect { text ->
                suppressNextClipboardText = text
                clipboardManager.setPrimaryClip(ClipData.newPlainText("MPClipboard", text))
            }
        }
    }

    fun stop() {
        if (!isStarted) {
            return
        }
        isStarted = false
        clipboardManager.removePrimaryClipChangedListener(clipboardListener)
        incomingJob?.cancel()
        incomingJob = null
        connectionManager.stop()
    }

    fun close() {
        stop()
        connectionManager.close()
        scope.cancel()
    }

    private fun clipboardText(): String? {
        val clip = clipboardManager.primaryClip ?: return null
        if (clip.itemCount <= 0) {
            return null
        }
        return clip.getItemAt(0).coerceToText(appContext)?.toString()?.takeIf { it.isNotEmpty() }
    }

    companion object {
        @Volatile
        private var shared: MPClipboardAndroidSync? = null

        fun startShared(context: Context): MPClipboardAndroidSync {
            return synchronized(this) {
                val sync = shared ?: MPClipboardAndroidSync(context.applicationContext).also { sync ->
                    shared = sync
                }
                sync.start()
                sync
            }
        }

        fun stopShared() {
            synchronized(this) {
                shared?.stop()
            }
        }

        fun closeShared() {
            synchronized(this) {
                shared?.close()
                shared = null
            }
        }
    }
}
