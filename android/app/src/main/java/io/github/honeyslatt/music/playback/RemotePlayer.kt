package io.github.honeyslatt.music.playback

import android.net.Uri
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.DeviceInfo
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import io.github.honeyslatt.music.core.connect.Command
import io.github.honeyslatt.music.ui.components.at
import kotlin.math.roundToInt

/**
 * The device played on, as Android's media session sees a player: the
 * notification, the lock screen and the control centre show what it
 * plays, their buttons control it, and the volume keys set its volume,
 * as Spotify does with its devices.
 */
@UnstableApi
class RemotePlayer(private val playback: Playback) : SimpleBasePlayer(Looper.getMainLooper()) {
    /** What the device plays changed. */
    fun changed() = invalidateState()

    private fun send(command: Command): ListenableFuture<*> {
        playback.remote.value?.let { playback.devices.send(it.device.id, command) }
        return Futures.immediateVoidFuture()
    }

    private val volume: Float? get() = playback.remote.value?.playing?.volume

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
        )
        if (seekable) commands.add(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM)
        val v = p?.volume
        if (v != null) {
            commands.addAll(
                Player.COMMAND_GET_DEVICE_VOLUME,
                Player.COMMAND_SET_DEVICE_VOLUME_WITH_FLAGS,
                Player.COMMAND_ADJUST_DEVICE_VOLUME_WITH_FLAGS,
            )
        }
        val state = State.Builder().setAvailableCommands(commands.build())
        if (v != null) {
            state.setDeviceInfo(DeviceInfo.Builder(DeviceInfo.PLAYBACK_TYPE_REMOTE).setMaxVolume(VOLUME_STEPS).build())
                .setDeviceVolume((v * VOLUME_STEPS).roundToInt())
        }
        if (t == null) return state.setPlaybackState(Player.STATE_IDLE).build()
        val metadata = MediaMetadata.Builder()
            .setTitle(t.title)
            .setArtist(t.artists.joinToString(", "))
            .setAlbumTitle(t.album)
            .setArtworkUri(t.artwork?.at(600)?.let(Uri::parse))
            .build()
        val id = "${t.id.provider}:${t.id.id}"
        val item = MediaItemData.Builder(id)
            .setMediaItem(MediaItem.Builder().setMediaId(id).setMediaMetadata(metadata).build())
            .setMediaMetadata(metadata)
            .setDurationUs(p.durationMs?.let { it.toLong() * 1000 } ?: C.TIME_UNSET)
            .setIsSeekable(seekable)
            .build()
        val position = playback.position
        return state
            .setPlaylist(listOf(item))
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
            else -> Command.Seek(positionMs.coerceAtLeast(0).toULong())
        },
    )

    override fun handleSetDeviceVolume(deviceVolume: Int, flags: Int) =
        send(Command.SetVolume((deviceVolume.toFloat() / VOLUME_STEPS).coerceIn(0f, 1f)))

    override fun handleIncreaseDeviceVolume(flags: Int) = step(1)

    override fun handleDecreaseDeviceVolume(flags: Int) = step(-1)

    private fun step(by: Int): ListenableFuture<*> {
        val v = volume ?: return Futures.immediateVoidFuture()
        return send(Command.SetVolume((v + by.toFloat() / VOLUME_STEPS).coerceIn(0f, 1f)))
    }

    private companion object {
        /** The volume keys move it a twentieth at a time. */
        const val VOLUME_STEPS = 20
    }
}
