package dev.ibylich.mpclipboard

import android.app.Application
import dev.ibylich.mpclipboard.widget.MPClipboardWidgetProvider
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch

class MPClipboardApplication : Application() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    override fun onCreate() {
        super.onCreate()

        val store = Store.from(this)
        MPClipboard.setConnectivityChangedCallback { connectivity ->
            scope.launch {
                store.writeConnectivity(connectivity)
                MPClipboardWidgetProvider.updateAll(this@MPClipboardApplication)
            }
        }
        MPClipboard.setTextReceivedCallback(AidlTransport::pushText)
        AidlTransport.setOnNewTextCallback(MPClipboard::pushText)

        scope.launch {
            store.config.collect { config ->
                MPClipboard.restart(config.host, config.token, config.id)
            }
        }
    }
}
