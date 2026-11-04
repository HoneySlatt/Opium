package io.github.honeyslatt.opium.playback

import android.net.Uri
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import io.github.honeyslatt.opium.core.connect.Command
import io.github.honeyslatt.opium.ui.components.at

/**
 * The device played on, as Android's media session sees a player: the
 * notification, the lock screen and the control centre show what it
 * plays, and their buttons control it, as Spotify does with its devices.
 * The volume keys stay the phone's, but in the app (see MainActivity).
 */
@UnstableApi
class RemotePlayer(private val playback: Playback, private val context: android.content.Context) : SimpleBasePlayer(Looper.getMainLooper()) {
    /** What the device plays changed. */
    fun changed() = invalidateState()

    private fun send(command: Command): ListenableFuture<*> {
        playback.remote.value?.let { playback.devices.send(it.device.id, command) }
        return Futures.immediateVoidFuture()
    }

    override fun getState(): State {
        val p = playback.remote.value?.playing
        val t = p?.track
        val seekable = p != null && !p.live && p.durationMs != null
        val commands = Player.Commands.Builder().addAll(
            Player.COMMAND_PLAY_PAUSE,
            Player.COMMAND_STOP,
            Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
            Player.COMMAND_GET_METADATA,
            Player.COMMAND_GET_TIMELINE,
            Player.COMMAND_SEEK_TO_NEXT,
            Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
            Player.COMMAND_SEEK_TO_PREVIOUS,
            Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
            Player.COMMAND_SEEK_TO_MEDIA_ITEM,
            // What the car picks goes to the device by the app's queue
            // (CarLibrary): taken here and left be.
            Player.COMMAND_SET_MEDIA_ITEM,
            Player.COMMAND_CHANGE_MEDIA_ITEMS,
            Player.COMMAND_PREPARE,
        )
        if (seekable) commands.add(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM)
        val state = State.Builder().setAvailableCommands(commands.build())
        if (t == null) return state.setPlaybackState(Player.STATE_IDLE).build()
        val metadata = MediaMetadata.Builder()
            .setTitle(t.title)
            .setArtist(t.artists.joinToString(", "))
            .setAlbumTitle(t.album)
            .setArtworkUri(t.artwork?.let { ArtProvider.uri(context, it.at(600)) })
            .build()
        val id = "${t.id.provider}:${t.id.id}"
        val item = MediaItemData.Builder(id)
            .setMediaItem(MediaItem.Builder().setMediaId(id).setMediaMetadata(metadata).build())
            .setMediaMetadata(metadata)
            .setDurationUs(p.durationMs?.let { it.toLong() * 1000 } ?: C.TIME_UNSET)
            .setIsSeekable(seekable)
            .build()
        // What comes there, for the car's Queue.
        val upcoming = playback.remote.value?.upcoming.orEmpty().mapIndexed { k, u ->
            val uid = "${u.id.provider}:${u.id.id}"
            val m = MediaMetadata.Builder().setTitle(u.title).setArtist(u.artists.joinToString(", ")).setAlbumTitle(u.album).build()
            MediaItemData.Builder("queue-$k-$uid")
                .setMediaItem(MediaItem.Builder().setMediaId(uid).setMediaMetadata(m).build())
                .setMediaMetadata(m)
                .setDurationUs(u.duration?.toMillis()?.let { it * 1000 } ?: C.TIME_UNSET)
                .build()
        }
        val position = playback.position
        return state
            .setPlaylist(listOf(item) + upcoming)
            .setCurrentMediaItemIndex(0)
            .setPlaybackState(Player.STATE_READY)
            .setPlayWhenReady(p.playing, Player.PLAY_WHEN_READY_CHANGE_REASON_REMOTE)
            .setContentPositionMs(
                if (p.playing) PositionSupplier.getExtrapolating(position, 1f) else PositionSupplier.getConstant(position),
            )
            .build()
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean) = send(if (playWhenReady) Command.Play else Command.Pause)

    override fun handleStop() = send(Command.Pause)

    override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int) = send(
        when (seekCommand) {
            Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> Command.Next
            Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> Command.Previous
            // A song picked further down the queue.
            else -> if (mediaItemIndex > 0) Command.Jump((mediaItemIndex - 1).toUInt()) else Command.Seek(positionMs.coerceAtLeast(0).toULong())
        },
    )

    override fun handlePrepare(): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleSetMediaItems(mediaItems: MutableList<MediaItem>, startIndex: Int, startPositionMs: Long): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleAddMediaItems(index: Int, mediaItems: MutableList<MediaItem>): ListenableFuture<*> = Futures.immediateVoidFuture()
}
