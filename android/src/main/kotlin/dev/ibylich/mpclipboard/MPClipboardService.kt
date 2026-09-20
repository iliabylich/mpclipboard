package dev.ibylich.mpclipboard

import android.app.Service
import android.content.Intent
import android.os.IBinder

class MPClipboardService : Service() {
    override fun onBind(intent: Intent?): IBinder = AidlTransport.binder

    override fun onDestroy() {
        AidlTransport.onServiceDestroyed()
        super.onDestroy()
    }
}
