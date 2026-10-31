package io.github.honeyslatt.music.data

import android.content.Context
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import io.github.honeyslatt.music.core.Connect
import io.github.honeyslatt.music.core.ConnectListener
import io.github.honeyslatt.music.core.connect.Command
import io.github.honeyslatt.music.core.connect.Device
import io.github.honeyslatt.music.core.connect.Event
import io.github.honeyslatt.music.core.model.Handoff
import io.github.honeyslatt.music.core.connect.Peer
import io.github.honeyslatt.music.core.connect.Playing
import io.github.honeyslatt.music.core.model.Track
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

/** Pairing with another device, from either end. */
sealed interface Pairing {
    val device: Device

    /** It shows a code, to type in here. */
    data class Asking(override val device: Device) : Pairing

    /** It asks to pair: this code is to be typed in there. */
    data class Showing(override val device: Device, val code: String) : Pairing
}

/**
 * The user's other devices on the Wi-Fi, through Music Connect (the Rust
 * core's): who is there, what each one plays, pairing. What they ask of
 * this phone's player goes to [heard], on the main thread.
 */
class Devices(context: Context) {
    private val main = Handler(Looper.getMainLooper())
    private val wifi = context.applicationContext.getSystemService(WifiManager::class.java)

    // Android drops the network's multicast to save power: the devices'
    // announcements need it let through, but only while they are looked
    // for (see [watch]).
    private val multicast = wifi.createMulticastLock("music-connect").apply { setReferenceCounted(false) }

    private val _peers = MutableStateFlow<List<Peer>>(emptyList())

    /** The devices seen or paired; their positions as of [peersAt]. */
    val peers: StateFlow<List<Peer>> = _peers.asStateFlow()

    /** When [peers] was read, in [SystemClock.elapsedRealtime] ms. */
    var peersAt = 0L
        private set

    private val _pairing = MutableStateFlow<Pairing?>(null)
    val pairing: StateFlow<Pairing?> = _pairing.asStateFlow()

    private val _notices = MutableSharedFlow<String>(extraBufferCapacity = 4)

    /** What to tell the user: paired, or why not. */
    val notices: SharedFlow<String> = _notices.asSharedFlow()

    /** A device asked for something, handed its music over, or what is kept in common changed. */
    var heard: ((Event) -> Unit)? = null

    /** The themes a device shares, all of them. */
    var themesShared: ((List<io.github.honeyslatt.music.core.connect.SharedTheme>) -> Unit)? = null

    private val node = Connect(
        android.provider.Settings.Global.getString(context.contentResolver, android.provider.Settings.Global.DEVICE_NAME)
            ?.takeIf { it.isNotBlank() } ?: Build.MODEL,
        context.filesDir.path,
        java.io.File(context.filesDir, "history.json").path,
        java.io.File(context.filesDir, "session.json").path,
        object : ConnectListener {
            override fun heard(event: Event) {
                main.post { take(event) }
            }
        },
    )

    private val _history = MutableStateFlow(node.history())

    /** The songs played lately, on this phone and the other devices, the last first. */
    val history: StateFlow<List<Track>> = _history.asStateFlow()

    /** `track` played here; the devices are told. */
    fun played(track: Track) {
        node.played(track)
        _history.value = node.history()
    }

    /** The history cleared, on every device. */
    fun clearHistory() {
        node.clearHistory()
        _history.value = node.history()
    }

    /** Where the music last stopped, here or on another device. */
    fun session(): io.github.honeyslatt.music.core.LastSession? = node.session()

    /** The music stopped here, or moved on, at `session`; written before it returns: off the main thread. */
    fun left(session: Handoff) = node.left(session)

    init {
        read()
    }

    private fun read() {
        _peers.value = node.peers()
        peersAt = SystemClock.elapsedRealtime()
        watch()
    }

    private var onScreen = false
    private var playing = false

    /** Until when, after music started in the background, a paired device is looked out for. */
    private var lookingUntil = 0L
    private val lookedEnough = Runnable { watch() }

    /**
     * Let the announcements through while the app is on screen (a device
     * may be there to pair), or for a minute after music starts in the
     * background while a paired device isn't connected yet; not otherwise:
     * a connection made needs none, a device away (the computer off) won't
     * come for being looked for all along, and Wi-Fi's filter saves battery.
     */
    private fun watch() {
        val missing = _peers.value.any { it.paired && !it.connected }
        val lately = SystemClock.elapsedRealtime() < lookingUntil
        if (onScreen || (playing && missing && lately)) multicast.acquire() else multicast.release()
    }

    private fun take(event: Event) {
        when (event) {
            Event.Changed -> read()
            is Event.AskCode -> _pairing.value = Pairing.Asking(event.device)
            is Event.ShowCode -> _pairing.value = Pairing.Showing(event.device, event.code)
            is Event.Paired -> {
                _pairing.value = null
                read()
                _notices.tryEmit(event.error?.let { "Not paired with ${event.device.name}: $it" } ?: "Paired with ${event.device.name}")
            }
            is Event.Asked, is Event.Handed -> heard?.invoke(event)
            is Event.Themes -> themesShared?.invoke(event.themes)
            Event.Synced -> {
                _history.value = node.history()
                heard?.invoke(event)
            }
        }
    }

    /** The app came on screen, or left it. */
    fun looking(on: Boolean) {
        onScreen = on
        watch()
    }

    /** Music started or stopped playing on this phone. */
    fun playing(on: Boolean) {
        if (on && !playing) {
            lookingUntil = SystemClock.elapsedRealtime() + LOOK_FOR
            main.removeCallbacks(lookedEnough)
            main.postDelayed(lookedEnough, LOOK_FOR)
        }
        playing = on
        watch()
    }

    fun notice(text: String) {
        _notices.tryEmit(text)
    }

    fun name(id: String): String = _peers.value.find { it.device.id == id }?.device?.name ?: "another device"

    fun publish(playing: Playing) = node.publish(playing)

    fun publishUpcoming(upcoming: List<Track>) = node.publishUpcoming(upcoming)

    fun send(to: String, command: Command) = node.send(to, command)

    fun handOff(to: String, handoff: Handoff) = node.handOff(to, handoff)

    fun pair(id: String) = node.pair(id)

    fun enterCode(code: String) = node.enterCode(code)

    fun cancelPairing() {
        node.cancelPairing()
        _pairing.value = null
    }

    fun forget(id: String) = node.forget(id)
}

/** How long a paired device is looked out for once music starts in the background. */
private const val LOOK_FOR = 60_000L
