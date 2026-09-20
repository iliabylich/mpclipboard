package dev.ibylich.mpclipboard.widget

import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.widget.RemoteViews
import dev.ibylich.mpclipboard.Ffi.Connectivity
import dev.ibylich.mpclipboard.R
import dev.ibylich.mpclipboard.Store
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

class MPClipboardWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(
        context: Context,
        appWidgetManager: AppWidgetManager,
        appWidgetIds: IntArray,
    ) {
        val pendingResult = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                updateAll(context)
            } finally {
                pendingResult.finish()
            }
        }
    }

    companion object {
        suspend fun updateAll(context: Context) {
            val appContext = context.applicationContext
            val connectivity = Store.from(appContext).connectivity.first()
            val appWidgetManager = AppWidgetManager.getInstance(appContext)
            val componentName = ComponentName(appContext, MPClipboardWidgetProvider::class.java)
            val appWidgetIds = appWidgetManager.getAppWidgetIds(componentName)
            updateWidgets(appContext, appWidgetManager, appWidgetIds, connectivity)
        }

        private fun updateWidgets(
            context: Context,
            appWidgetManager: AppWidgetManager,
            appWidgetIds: IntArray,
            connectivity: Connectivity,
        ) {
            for (appWidgetId in appWidgetIds) {
                appWidgetManager.updateAppWidget(
                    appWidgetId,
                    remoteViews(context, connectivity),
                )
            }
        }

        private fun remoteViews(context: Context, connectivity: Connectivity): RemoteViews {
            val icon = when (connectivity) {
                Connectivity.Connecting -> R.drawable.mpclipboard_widget_connecting
                Connectivity.Connected -> R.drawable.mpclipboard_widget_connected
                Connectivity.Disconnected -> R.drawable.mpclipboard_widget_disconnected
            }
            val remoteViews = RemoteViews(context.packageName, R.layout.mpclipboard_widget)
            remoteViews.setImageViewResource(
                R.id.mpclipboard_widget_icon,
                icon,
            )
            return remoteViews
        }
    }
}
