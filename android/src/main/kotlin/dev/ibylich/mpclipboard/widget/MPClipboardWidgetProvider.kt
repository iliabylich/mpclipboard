package dev.ibylich.mpclipboard.widget

import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.widget.RemoteViews
import dev.ibylich.mpclipboard.Ffi.Connectivity
import dev.ibylich.mpclipboard.R
import dev.ibylich.mpclipboard.Store

class MPClipboardWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(
        context: Context,
        appWidgetManager: AppWidgetManager,
        appWidgetIds: IntArray,
    ) {
        val connectivity = Store.from(context).connectivity.value
        updateWidgets(context, appWidgetManager, appWidgetIds, connectivity)
    }

    companion object {
        fun updateAll(context: Context, connectivity: Connectivity) {
            val appWidgetManager = AppWidgetManager.getInstance(context)
            val componentName = ComponentName(context, MPClipboardWidgetProvider::class.java)
            val appWidgetIds = appWidgetManager.getAppWidgetIds(componentName)
            updateWidgets(context, appWidgetManager, appWidgetIds, connectivity)
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
