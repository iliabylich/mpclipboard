package dev.mpclipboard.android.widget

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.widget.RemoteViews
import dev.mpclipboard.android.Connectivity
import dev.mpclipboard.android.MPClipboardStore
import dev.mpclipboard.android.R
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
                val connectivity = MPClipboardStore.from(context).connectivity.first()
                updateWidgets(context, appWidgetManager, appWidgetIds, connectivity)
            } finally {
                pendingResult.finish()
            }
        }
    }

    companion object {
        fun updateAll(context: Context, connectivity: Connectivity) {
            val appContext = context.applicationContext
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
            val isConnected = connectivity == Connectivity.Connected
            val remoteViews = RemoteViews(context.packageName, R.layout.mpclipboard_widget)
            remoteViews.setImageViewResource(
                R.id.mpclipboard_widget_icon,
                if (isConnected) {
                    R.drawable.mpclipboard_widget_connected
                } else {
                    R.drawable.mpclipboard_widget_disconnected
                },
            )
            remoteViews.setContentDescription(
                R.id.mpclipboard_widget_icon,
                context.getString(
                    if (isConnected) {
                        R.string.mpclipboard_widget_connected
                    } else {
                        R.string.mpclipboard_widget_disconnected
                    },
                ),
            )
            remoteViews.setOnClickPendingIntent(
                R.id.mpclipboard_widget_root,
                launchAppPendingIntent(context),
            )
            return remoteViews
        }

        private fun launchAppPendingIntent(context: Context): PendingIntent? {
            val intent = context.packageManager.getLaunchIntentForPackage(context.packageName)
                ?: return null
            val flags = PendingIntent.FLAG_UPDATE_CURRENT or if (Build.VERSION.SDK_INT >= 23) {
                PendingIntent.FLAG_IMMUTABLE
            } else {
                0
            }
            return PendingIntent.getActivity(context, 0, intent, flags)
        }
    }
}
