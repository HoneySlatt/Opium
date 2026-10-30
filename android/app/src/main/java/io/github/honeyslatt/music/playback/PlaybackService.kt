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

/**
 * The media session Android draws the notification, the lock screen and
 * the car from. Next and previous go through the app's queue: the player
 * itself only holds the current song and the next.
 */
@UnstableApi
class PlaybackService : MediaSessionService() {
    private var session: MediaSession? = null

    override fun onCreate() {
        super.onCreate()
        val playback = (application as MusicApp).playback
        val player = object : ForwardingPlayer(playback.player) {
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
        val open = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        session = MediaSession.Builder(this, player).setSessionActivity(open).build()
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo) = session

    /** Swiped away while paused: nothing to keep running for. */
    override fun onTaskRemoved(rootIntent: Intent?) {
        val player = session?.player
        if (player == null || !player.playWhenReady || player.mediaItemCount == 0) stopSelf()
    }

    override fun onDestroy() {
        session?.release()
        session = null
        super.onDestroy()
    }
}
