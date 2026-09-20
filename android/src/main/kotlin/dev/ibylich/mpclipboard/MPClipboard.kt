package dev.ibylich.mpclipboard

import android.os.Looper
import android.os.MessageQueue

object MPClipboard {
    private var current: Ffi.Client? = null

    private var connectivityChangedCallback: ((Ffi.Connectivity) -> Unit)? = null
    private var textReceivedCallback: ((String) -> Unit)? = null

    fun start(host: String, token: String, id: String) {
        checkMainThread()
        val mpclipboard = Ffi.Client.new(host, token, id) ?: return
        current = mpclipboard
        registerFileDescriptorListener(mpclipboard)
    }

    fun restart(host: String, token: String, id: String) {
        checkMainThread()
        closeCurrentIfPresent()
        val mpclipboard = Ffi.Client.new(host, token, id) ?: return
        current = mpclipboard
        registerFileDescriptorListener(mpclipboard)
    }

    fun pushText(text: String) {
        checkMainThread()
        current?.pushText(text)
    }

    fun setConnectivityChangedCallback(
        callback: (Ffi.Connectivity) -> Unit,
    ) {
        checkMainThread()
        connectivityChangedCallback = callback
    }

    fun setTextReceivedCallback(callback: (String) -> Unit) {
        checkMainThread()
        textReceivedCallback = callback
    }

    private fun registerFileDescriptorListener(mpclipboard: Ffi.Client) {
        Looper.getMainLooper().queue.addOnFileDescriptorEventListener(
            mpclipboard.fd,
            MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT,
        ) { _, events ->
            if (mpclipboard !== current) {
                return@addOnFileDescriptorEventListener 0
            }
            if ((events and MessageQueue.OnFileDescriptorEventListener.EVENT_ERROR) != 0) {
                closeCurrentIfPresent()
                return@addOnFileDescriptorEventListener 0
            }
            if ((events and MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT) != 0) {
                read(mpclipboard)
            }
            MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT
        }
    }

    private fun read(mpclipboard: Ffi.Client) {
        val output = mpclipboard.read() ?: return
        output.connectivity?.let { connectivity ->
            connectivityChangedCallback?.invoke(connectivity)
        }
        output.text?.let { text ->
            textReceivedCallback?.invoke(text)
        }
    }

    private fun closeCurrentIfPresent() {
        val mpclipboard = current ?: return
        current = null
        Looper.getMainLooper().queue.removeOnFileDescriptorEventListener(mpclipboard.fd)
        mpclipboard.close()
    }

    private fun checkMainThread() {
        check(Looper.myLooper() === Looper.getMainLooper()) {
            "MPClipboard must be called from the main thread"
        }
    }
}
