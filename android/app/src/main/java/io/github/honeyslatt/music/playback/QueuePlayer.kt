package io.github.honeyslatt.music.playback

import android.content.Context
import androidx.media3.common.C
import androidx.media3.common.ForwardingSimpleBasePlayer
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.components.at
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/**
 * This phone's player as the media session shows it: the song playing
 * and the whole queue after it (its hundred songs at most), rather than
 * the song and the next the player holds, for the car's Queue to list
 * what comes. What is asked of it goes through the app's queue: next,
 * previous, a song picked in the queue, Play with nothing made playable
 * yet. What the car picks to play is played by [CarLibrary] before it
 * comes here, so it is taken and left be.
 */
@UnstableApi
class QueuePlayer(
    private val playback: Playback,
    private val context: Context,
    scope: CoroutineScope,
) : ForwardingSimpleBasePlayer(playback.player) {
    init {
        scope.launch { playback.upcoming.collect { invalidateState() } }
    }

    private val queueCommands = listOf(
        Player.COMMAND_SEEK_TO_NEXT,
        Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
        Player.COMMAND_SEEK_TO_PREVIOUS,
        Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
        Player.COMMAND_SEEK_TO_MEDIA_ITEM,
        Player.COMMAND_SET_MEDIA_ITEM,
        Player.COMMAND_CHANGE_MEDIA_ITEMS,
        Player.COMMAND_PREPARE,
    )

    override fun getState(): State {
        val state = super.getState()
        val commands = state.availableCommands.buildUpon()
            .addAll(*queueCommands.toIntArray())
            // Shuffle and repeat are the app's queue's: buttons of their own.
            .removeAll(Player.COMMAND_SET_SHUFFLE_MODE, Player.COMMAND_SET_REPEAT_MODE)
            .build()
        val playlist = state.playlist
        // Nothing made playable yet: nothing to list.
        if (playlist.isEmpty()) return state.buildUpon().setAvailableCommands(commands).build()
        val current = playlist[state.currentMediaItemIndex.coerceIn(0, playlist.lastIndex)]
        val upcoming = playback.upcoming.value.mapIndexed(::queued)
        return state.buildUpon()
            .setAvailableCommands(commands)
            .setPlaylist(listOf(current) + upcoming)
            .setCurrentMediaItemIndex(0)
            .build()
    }

    /** The `k`-th song to come, as the queue lists it. */
    private fun queued(k: Int, track: Track): MediaItemData {
        val id = "${track.id.provider}:${track.id.id}"
        val metadata = MediaMetadata.Builder()
            .setTitle(track.title)
            .setArtist(track.artists.joinToString(", "))
            .setAlbumTitle(track.album)
            .setArtworkUri(track.artwork?.let { ArtProvider.uri(context, it.at(240)) })
            .build()
        return MediaItemData.Builder("queue-$k-$id")
            .setMediaItem(MediaItem.Builder().setMediaId(id).setMediaMetadata(metadata).build())
            .setMediaMetadata(metadata)
            .setDurationUs(track.duration?.toMillis()?.let { it * 1000 } ?: C.TIME_UNSET)
            .build()
    }

    override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int): ListenableFuture<*> {
        when (seekCommand) {
            Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> playback.next()
            Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> playback.previous()
            // A song picked further down the queue.
            else -> if (mediaItemIndex > 0) playback.jump(mediaItemIndex - 1) else playback.seekTo(positionMs.coerceAtLeast(0))
        }
        return Futures.immediateVoidFuture()
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        val player = player
        when {
            !playWhenReady -> return super.handleSetPlayWhenReady(false)
            // A song picked is on its way, and plays when it comes.
            playback.loading.value -> {}
            // Play from the earbuds, the lock screen or the car with nothing
            // made playable yet (the app just opened on what played last),
            // or at the queue's end: the app's own Play, which sees to it.
            player.mediaItemCount == 0 || player.playbackState == Player.STATE_IDLE || player.playbackState == Player.STATE_ENDED -> playback.toggle()
            else -> return super.handleSetPlayWhenReady(true)
        }
        return Futures.immediateVoidFuture()
    }

    // Media3 prepares an idle player before Play; the app does that its own way.
    override fun handlePrepare(): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleSetMediaItems(mediaItems: MutableList<MediaItem>, startIndex: Int, startPositionMs: Long): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleAddMediaItems(index: Int, mediaItems: MutableList<MediaItem>): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleRemoveMediaItems(fromIndex: Int, toIndex: Int): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleMoveMediaItems(fromIndex: Int, toIndex: Int, newIndex: Int): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleReplaceMediaItems(fromIndex: Int, toIndex: Int, mediaItems: MutableList<MediaItem>): ListenableFuture<*> = Futures.immediateVoidFuture()
}
