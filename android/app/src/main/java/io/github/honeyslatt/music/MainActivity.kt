package io.github.honeyslatt.music

// Step 0: can this phone sign in to Apple Music, license a song through its
// own Widevine (MediaDrm), and play it with ExoPlayer? A throwaway screen.

import android.annotation.SuppressLint
import android.media.MediaDrm
import android.net.Uri
import android.os.Bundle
import android.webkit.CookieManager
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.ByteArrayDataSource
import androidx.media3.datasource.DataSource
import androidx.media3.datasource.DataSpec
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.datasource.TransferListener
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.drm.DefaultDrmSessionManager
import androidx.media3.exoplayer.drm.ExoMediaDrm
import androidx.media3.exoplayer.drm.FrameworkMediaDrm
import androidx.media3.exoplayer.drm.HttpMediaDrmCallback
import androidx.media3.exoplayer.drm.MediaDrmCallback
import androidx.media3.exoplayer.hls.HlsMediaSource
import io.github.honeyslatt.music.core.Apple
import io.github.honeyslatt.music.core.ProtectedTrack
import io.github.honeyslatt.music.core.Tls
import java.io.File
import java.util.UUID
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

/** Breathe (In the Air), which a gapless album follows with On the Run. */
private const val SONG = "1065973704"
private const val NEXT_SONG = "1065973705"

class MainActivity : ComponentActivity() {
    private var player: ExoPlayer? = null
    private var tapped = 0L

    private fun since() = android.os.SystemClock.elapsedRealtime() - tapped

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Tls.init(applicationContext)
        setContent { Spike() }
    }

    override fun onDestroy() {
        player?.release()
        super.onDestroy()
    }

    private val tokenFile get() = File(filesDir, "apple-token")

    @Composable
    private fun Spike() {
        var log by remember { mutableStateOf(listOf("Widevine: ${widevine()}")) }
        var signingIn by remember { mutableStateOf(false) }
        var apple by remember { mutableStateOf<Apple?>(null) }
        val scope = rememberCoroutineScope()
        fun say(line: String) {
            log = log + line
        }

        if (signingIn) {
            SignIn { token ->
                tokenFile.writeText(token)
                signingIn = false
                say("Signed in: token kept")
            }
            return
        }
        LaunchedEffect(Unit) {
            while (true) {
                player?.let { p ->
                    if (p.isPlaying) {
                        val format = p.audioFormat
                        log = log.filterNot { it.startsWith("▶") } +
                            "▶ ${p.currentPosition / 1000} s · ${format?.sampleMimeType} " +
                            "${format?.sampleRate} Hz · ${format?.channelCount} ch"
                    }
                }
                delay(1000)
            }
        }
        Column(
            Modifier.fillMaxSize().background(Color.Black).padding(24.dp, 48.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Button("1. Sign in to Apple Music") { signingIn = true }
            Button("2. Connect") {
                scope.launch {
                    try {
                        val a = Apple.connect(tokenFile.readText())
                        apple = a
                        say("Connected · storefront ${a.storefront()}")
                    } catch (e: Exception) {
                        say("Connect failed: $e")
                    }
                }
            }
            Button("3. Play Breathe") {
                val a = apple ?: return@Button say("Connect first")
                tapped = android.os.SystemClock.elapsedRealtime()
                scope.launch {
                    try {
                        val track = a.protected(SONG) ?: return@launch say("No protected asset")
                        say("Asset · ${track.kbps()} kb/s · +${since()} ms")
                        play(listOf(track)) { say(it) }
                    } catch (e: Exception) {
                        say("Play failed: $e")
                    }
                }
            }
            Button("4. Seek +60 s") { player?.let { it.seekTo(it.currentPosition + 60_000) } }
            Button("5. Breathe → On the Run, near the join") {
                val a = apple ?: return@Button say("Connect first")
                scope.launch {
                    try {
                        val tracks = listOf(SONG, NEXT_SONG).map { a.protected(it) ?: error("no asset") }
                        tracks.forEach { say("Trim · ${it.priming()} before, ${it.padding()} after") }
                        play(tracks) { say(it) }
                        // Ten seconds before Breathe ends.
                        player?.seekTo(0, 159_000)
                    } catch (e: Exception) {
                        say("Play failed: $e")
                    }
                }
            }
            log.forEach { BasicText(it, style = TextStyle(Color.White, 14.sp)) }
        }
    }

    @Composable
    private fun Button(label: String, onClick: () -> Unit) {
        BasicText(
            label,
            Modifier.background(Color(0xFF7C3AED)).clickable(onClick = onClick).padding(16.dp, 12.dp),
            style = TextStyle(Color.White, 16.sp),
        )
    }

    /** Apple's sign-in page; done once its cookies hold the user's token. */
    @SuppressLint("SetJavaScriptEnabled")
    @Composable
    private fun SignIn(done: (String) -> Unit) {
        LaunchedEffect(Unit) {
            while (true) {
                val cookies = CookieManager.getInstance().getCookie("https://music.apple.com").orEmpty()
                cookies.split(';')
                    .map { it.trim() }
                    .firstOrNull { it.startsWith("media-user-token=") && it.length > 17 }
                    ?.let { return@LaunchedEffect done(it.removePrefix("media-user-token=")) }
                delay(500)
            }
        }
        AndroidView(
            factory = {
                WebView(it).apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    webViewClient = WebViewClient()
                    loadUrl("https://music.apple.com/signin")
                }
            },
            modifier = Modifier.fillMaxSize(),
        )
    }

    /**
     * An Apple song as ExoPlayer plays it: its playlist from the core, its
     * audio from Apple, its licence through the core, and its encoder's
     * priming and padding left out so that songs follow without a gap.
     */
    @androidx.annotation.OptIn(UnstableApi::class)
    private fun source(track: ProtectedTrack, key: String, say: (String) -> Unit): HlsMediaSource {
        val playlistUri = Uri.parse("music-playlist://apple/$key.m3u8")
        val playlist = track.playlist().toByteArray()
        val http = DefaultHttpDataSource.Factory()
        // The playlist comes from the core; the audio from Apple's servers.
        val sources = DataSource.Factory {
            object : DataSource {
                private var inner: DataSource? = null
                private val listeners = mutableListOf<TransferListener>()

                override fun addTransferListener(listener: TransferListener) {
                    listeners += listener
                }

                override fun open(spec: DataSpec): Long {
                    val source = if (spec.uri == playlistUri) ByteArrayDataSource(playlist) else http.createDataSource()
                    listeners.forEach(source::addTransferListener)
                    inner = source
                    return source.open(spec)
                }

                override fun read(buffer: ByteArray, offset: Int, length: Int) =
                    inner!!.read(buffer, offset, length)

                override fun getUri(): Uri? = inner?.uri

                override fun close() {
                    inner?.close()
                    inner = null
                }
            }
        }
        val licences = object : MediaDrmCallback {
            override fun executeProvisionRequest(uuid: UUID, request: ExoMediaDrm.ProvisionRequest) =
                HttpMediaDrmCallback(null, http).executeProvisionRequest(uuid, request)

            override fun executeKeyRequest(uuid: UUID, request: ExoMediaDrm.KeyRequest): MediaDrmCallback.Response {
                runOnUiThread { say("Licence asked · +${since()} ms") }
                val licence = runBlocking { track.license(request.data) }
                runOnUiThread { say("Licence: ${licence.size} bytes · +${since()} ms") }
                return MediaDrmCallback.Response(licence)
            }
        }
        val drm = DefaultDrmSessionManager.Builder()
            .setUuidAndExoMediaDrmProvider(C.WIDEVINE_UUID, FrameworkMediaDrm.DEFAULT_PROVIDER)
            .build(licences)
        return HlsMediaSource.Factory(sources)
            .setExtractorFactory(Trimmed.Factory(track.priming().toInt(), track.padding().toInt()))
            .setDrmSessionManagerProvider { drm }
            .createMediaSource(
                MediaItem.Builder().setUri(playlistUri).setMimeType(MimeTypes.APPLICATION_M3U8).build(),
            )
    }

    private fun widevine(): String = try {
        val drm = MediaDrm(C.WIDEVINE_UUID)
        val level = drm.getPropertyString("securityLevel")
        val version = drm.getPropertyString(MediaDrm.PROPERTY_VERSION)
        drm.close()
        "$level · $version"
    } catch (e: Exception) {
        "unavailable: $e"
    }

    @androidx.annotation.OptIn(UnstableApi::class)
    private fun play(tracks: List<ProtectedTrack>, say: (String) -> Unit) {
        val sources = tracks.mapIndexed { i, track -> source(track, "$i", say) }
        player?.release()
        say("Player · +${since()} ms")
        player = ExoPlayer.Builder(this).build().apply {
            addListener(object : Player.Listener {
                override fun onPlayerError(error: PlaybackException) {
                    say("Player error: ${error.errorCodeName}: ${error.cause}")
                }

                override fun onIsPlayingChanged(isPlaying: Boolean) {
                    if (isPlaying) say("Playing · +${since()} ms")
                }
            })
            setMediaSources(sources)
            prepare()
            play()
        }
    }
}
