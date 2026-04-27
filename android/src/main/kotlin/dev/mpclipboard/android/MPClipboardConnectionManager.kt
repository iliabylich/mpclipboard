package dev.mpclipboard.android

import android.content.Context
import android.os.Looper
import android.os.MessageQueue
import java.io.FileDescriptor
import dev.mpclipboard.android.widget.MPClipboardWidgetProvider
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

class MPClipboardConnectionManager(
    context: Context,
    private val store: MPClipboardStore = MPClipboardStore.from(context),
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO),
) {
    private val appContext = context.applicationContext
    private val mutableConnectivity = MutableStateFlow(Connectivity.Disconnected)
    private val mutableIncomingText = MutableSharedFlow<String>(extraBufferCapacity = 16)
    private var client: MPClipboard? = null
    private var clientFd: FileDescriptor? = null
    private var startJob: Job? = null

    val connectivity: StateFlow<Connectivity> = mutableConnectivity
    val incomingText: SharedFlow<String> = mutableIncomingText

    fun start() {
        if (startJob?.isActive == true || client != null) {
            return
        }

        startJob = scope.launch {
            setConnectivity(Connectivity.Connecting)
            val config = store.config.first()

            if (!config.isComplete || !MPClipboard.init(appContext)) {
                setConnectivity(Connectivity.Disconnected)
                return@launch
            }

            val nextClient = MPClipboard.initialize(config.host, config.token, config.name)
            if (nextClient == null) {
                setConnectivity(Connectivity.Disconnected)
                return@launch
            }

            client = nextClient
            registerFileDescriptorListener(nextClient)
        }
    }

    fun stop() {
        startJob?.cancel()
        startJob = null
        unregisterFileDescriptorListener()
        client = null
        setConnectivity(Connectivity.Disconnected)
    }

    fun close() {
        stop()
        scope.cancel()
    }

    fun pushText(text: String): Boolean {
        return client?.pushText(text) ?: false
    }

    private fun registerFileDescriptorListener(mpclipboard: MPClipboard) {
        val fd = fileDescriptor(mpclipboard.getFd())
        clientFd = fd

        Looper.getMainLooper().queue.addOnFileDescriptorEventListener(
            fd,
            MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT,
        ) { _, events ->
            if ((events and MessageQueue.OnFileDescriptorEventListener.EVENT_ERROR) != 0) {
                client = null
                setConnectivity(Connectivity.Disconnected)
                return@addOnFileDescriptorEventListener 0
            }

            if ((events and MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT) != 0) {
                readOnce(mpclipboard)
            }

            if (client === mpclipboard) {
                MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT
            } else {
                0
            }
        }
    }

    private fun unregisterFileDescriptorListener() {
        clientFd?.let { fd ->
            Looper.getMainLooper().queue.removeOnFileDescriptorEventListener(fd)
        }
        clientFd = null
    }

    private fun readOnce(mpclipboard: MPClipboard) {
        when (val output = mpclipboard.read()) {
            is Output.ConnectivityChanged -> setConnectivity(output.connectivity)
            is Output.NewText -> mutableIncomingText.tryEmit(output.text)
            null -> Unit
        }
    }

    private fun setConnectivity(connectivity: Connectivity) {
        mutableConnectivity.value = connectivity
        MPClipboardWidgetProvider.updateAll(appContext, connectivity)
        scope.launch {
            store.saveConnectivity(connectivity)
        }
    }

    private fun fileDescriptor(fd: Int): FileDescriptor {
        val fileDescriptor = FileDescriptor()
        val setInt = FileDescriptor::class.java.getDeclaredMethod("setInt$", Int::class.javaPrimitiveType)
        setInt.isAccessible = true
        setInt.invoke(fileDescriptor, fd)
        return fileDescriptor
    }
}
