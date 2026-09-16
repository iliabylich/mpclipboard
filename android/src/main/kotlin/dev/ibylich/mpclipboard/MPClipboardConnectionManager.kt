package dev.ibylich.mpclipboard

import android.content.Context
import android.os.Looper
import android.os.MessageQueue
import java.io.FileDescriptor
import dev.ibylich.mpclipboard.widget.MPClipboardWidgetProvider
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

class MPClipboardConnectionManager(
    context: Context,
    private val store: MPClipboardStore = MPClipboardStore.from(context),
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate),
) {
    private val appContext = context.applicationContext
    private val mutableConnectivity = MutableStateFlow(Connectivity.Disconnected)
    private val mutableIncomingText = MutableSharedFlow<String>(extraBufferCapacity = 16)
    private var client: MPClipboard? = null
    private var clientFd: FileDescriptor? = null
    private var configJob: Job? = null
    private var activeConfig: MPClipboardConfig? = null

    val connectivity: StateFlow<Connectivity> = mutableConnectivity
    val incomingText: SharedFlow<String> = mutableIncomingText

    fun start() {
        if (configJob?.isActive == true) {
            return
        }

        configJob = scope.launch {
            store.config.collect { config ->
                applyConfig(config)
            }
        }
    }

    fun stop() {
        configJob?.cancel()
        configJob = null
        activeConfig = null
        closeClient()
        setConnectivity(Connectivity.Disconnected)
    }

    fun close() {
        stop()
        scope.cancel()
    }

    fun pushText(text: String): PushResult {
        val currentClient = client ?: return PushResult.Dropped
        return try {
            currentClient.pushText(text)
        } catch (_: RuntimeException) {
            closeClient()
            setConnectivity(Connectivity.Disconnected)
            PushResult.Dropped
        }
    }

    private fun applyConfig(config: MPClipboardConfig) {
        if (config == activeConfig) {
            return
        }

        activeConfig = config
        closeClient()
        if (!config.isComplete) {
            setConnectivity(Connectivity.Disconnected)
            return
        }

        setConnectivity(Connectivity.Connecting)
        try {
            if (!MPClipboard.init()) {
                setConnectivity(Connectivity.Disconnected)
                return
            }

            val nextClient = MPClipboard.initialize(config.host, config.token, config.name)
            if (nextClient == null) {
                setConnectivity(Connectivity.Disconnected)
                return
            }

            client = nextClient
            registerFileDescriptorListener(nextClient)
        } catch (_: RuntimeException) {
            closeClient()
            setConnectivity(Connectivity.Disconnected)
        }
    }

    private fun registerFileDescriptorListener(mpclipboard: MPClipboard) {
        val fd = fileDescriptor(mpclipboard.getFd())
        clientFd = fd

        Looper.getMainLooper().queue.addOnFileDescriptorEventListener(
            fd,
            MessageQueue.OnFileDescriptorEventListener.EVENT_INPUT,
        ) { _, events ->
            if ((events and MessageQueue.OnFileDescriptorEventListener.EVENT_ERROR) != 0) {
                closeClient()
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

    private fun closeClient() {
        unregisterFileDescriptorListener()
        client?.close()
        client = null
    }

    private fun readOnce(mpclipboard: MPClipboard) {
        try {
            when (val output = mpclipboard.read()) {
                is Output.ConnectivityChanged -> setConnectivity(output.connectivity)
                is Output.NewText -> mutableIncomingText.tryEmit(output.text)
                is Output.Both -> {
                    setConnectivity(output.connectivity)
                    mutableIncomingText.tryEmit(output.text)
                }
                null -> Unit
            }
        } catch (_: RuntimeException) {
            closeClient()
            setConnectivity(Connectivity.Disconnected)
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
