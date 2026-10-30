package io.github.honeyslatt.music.playback

import android.app.PendingIntent
import android.content.Intent
import androidx.media3.common.ForwardingPlayer
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import io.github.honeyslatt.music.MainActivity
import io.github.honeyslatt.music.MusicApp
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * The media session Android draws the notification, the lock screen and
 * the car from. Next and previous go through the app's queue: the player
 * itself only holds the current song and the next. While another device
 * is played on, the session shows and controls that one instead.
 */
@UnstableApi
class PlaybackService : MediaSessionService() {
    private var session: MediaSession? = null
    private val scope = kotlinx.coroutines.MainScope()

    override fun onCreate() {
        super.onCreate()
        val playback = (application as MusicApp).playback
        val here = queued(playback, playback.player)
        val far = RemotePlayer(playback)
        val there = queued(playback, far)
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
        val session = MediaSession.Builder(this, here).setSessionActivity(open).build()
        this.session = session
        scope.launch {
            playback.remote.collect { remote ->
                far.changed()
                val wanted = if (remote != null) there else here
                if (session.player !== wanted) session.player = wanted
            }
        }
    }

    /** `player`, its next and previous going through the app's queue. */
    private fun queued(playback: io.github.honeyslatt.music.playback.Playback, player: Player): Player =
        object : ForwardingPlayer(player) {
            private val queueCommands = listOf(
                Player.COMMAND_SEEK_TO_NEXT,
                Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_PREVIOUS,
                Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
            )

            override fun getAvailableCommands(): Player.Commands =
                super.getAvailableCommands().buildUpon().addAll(*queueCommands.toIntArray()).build()

            override fun isCommandAvailable(command: Int) = command in queueCommands || super.isCommandAvailable(command)

            override fun seekToNext() = playback.next()

            override fun seekToNextMediaItem() = playback.next()

            override fun seekToPrevious() = playback.previous()

            override fun seekToPreviousMediaItem() = playback.previous()
        }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo) = session

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
