package io.github.honeyslatt.music.playback

import android.app.PendingIntent
import android.content.Intent
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaLibraryService
import androidx.media3.session.MediaSession
import io.github.honeyslatt.music.MainActivity
import io.github.honeyslatt.music.MusicApp
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.launch

/**
 * The media session Android draws the notification, the lock screen and
 * the car from. Next and previous go through the app's queue: the player
 * itself only holds the current song and the next. While another device
 * is played on, the session shows and controls that one instead. Android
 * Auto browses the app's music through it (see [CarLibrary]).
 */
@UnstableApi
class PlaybackService : MediaLibraryService() {
    private var session: MediaLibrarySession? = null
    private val scope = kotlinx.coroutines.MainScope()

    override fun onCreate() {
        super.onCreate()
        val playback = (application as MusicApp).playback
        val here = QueuePlayer(playback, this, scope)
        val far = RemotePlayer(playback, applicationContext)
        val open = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        // The app's note in the media controls and the status bar, as
        // Apple Music has its own, rather than Media3's play button.
        setMediaNotificationProvider(
            androidx.media3.session.DefaultMediaNotificationProvider.Builder(this).build().apply {
                setSmallIcon(io.github.honeyslatt.music.R.drawable.ic_notification)
            },
        )
        val app = application as MusicApp
        val car = CarLibrary(app, app.scope)
        val session = MediaLibrarySession.Builder(this, here, car).setSessionActivity(open).build()
        this.session = session
        // Favourite, shuffle and repeat, as they change, in the car.
        scope.launch {
            combine(playback.now, playback.shuffleOn, playback.repeat) { _, _, _ -> }.collect { car.refresh() }
        }
        scope.launch {
            playback.remote.collect { remote ->
                far.changed()
                val wanted = if (remote != null) far else here
                if (session.player !== wanted) session.player = wanted
            }
        }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaLibrarySession? = session

    /**
     * Swiped away from the recent apps: the music stops, as an iPhone's
     * does, and comes back paused where it was. Another device played on
     * plays on: only this phone's player is paused.
     */
    override fun onTaskRemoved(rootIntent: Intent?) {
        val playback = (application as MusicApp).playback
        playback.player.pause()
        playback.keepSession()
        stopSelf()
    }

    override fun onDestroy() {
        scope.cancel()
        session?.release()
        session = null
        super.onDestroy()
    }
}
