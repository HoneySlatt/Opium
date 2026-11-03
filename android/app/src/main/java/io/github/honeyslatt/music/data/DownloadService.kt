package io.github.honeyslatt.music.data

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import io.github.honeyslatt.music.MainActivity
import io.github.honeyslatt.music.MusicApp
import io.github.honeyslatt.music.R
import io.github.honeyslatt.music.core.DownloadState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Keeps the app up while songs download, the screen off or another app
 * shown, with a notification saying how many are left; gone once none are.
 */
class DownloadService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var watching: Job? = null

    override fun onBind(intent: Intent?) = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        startForeground(ID, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        if (watching?.isActive != true) {
            val downloads = (application as MusicApp).downloads
            watching = scope.launch {
                while (downloads.busy()) {
                    getSystemService(NotificationManager::class.java).notify(ID, notification())
                    delay(1_000)
                }
                stopSelf()
            }
        }
        return START_NOT_STICKY
    }

    /** Android's time for downloading in the background is up for the day: what is left waits for the app. */
    override fun onTimeout(startId: Int, fgsType: Int) {
        stopSelf()
    }

    override fun onDestroy() {
        scope.cancel()
        running = false
        super.onDestroy()
    }

    private fun notification(): Notification {
        val states = (application as MusicApp).downloads.states.value.values
        val left = states.count { it is DownloadState.Queued || it is DownloadState.Coming }
        val part = states.filterIsInstance<DownloadState.Coming>().firstOrNull()?.part ?: 0f
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE,
        )
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("Downloading")
            .setContentText(if (left == 1) "1 song left" else "$left songs left")
            .setProgress(100, (part * 100).toInt(), left == 0)
            .setContentIntent(open)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .build()
    }

    companion object {
        private const val ID = 2
        private const val CHANNEL = "downloads"

        /** Set from the first start until the service is gone. */
        @Volatile
        private var running = false

        /** Songs are waiting: the service runs, if it doesn't already. */
        fun keepUp(context: Context) {
            if (running) return
            running = true
            context.getSystemService(NotificationManager::class.java).createNotificationChannel(
                NotificationChannel(CHANNEL, "Downloads", NotificationManager.IMPORTANCE_LOW),
            )
            runCatching {
                context.startForegroundService(Intent(context, DownloadService::class.java))
            }.onFailure { running = false }
        }
    }
}
