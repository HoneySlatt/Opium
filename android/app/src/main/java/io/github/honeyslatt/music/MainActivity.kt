package io.github.honeyslatt.music

import android.Manifest
import android.content.ComponentName
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.result.contract.ActivityResultContracts
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.ListenableFuture
import io.github.honeyslatt.music.playback.PlaybackService
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.ui.App
import io.github.honeyslatt.music.ui.connect.ConnectHost
import io.github.honeyslatt.music.ui.connect.Connecting
import io.github.honeyslatt.music.ui.connect.Welcome
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import io.github.honeyslatt.music.ui.theme.MusicTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private var controller: ListenableFuture<MediaController>? = null

    // The playing notification needs the permission since Android 13.
    private val notifications = registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    override fun onStart() {
        super.onStart()
        // Being connected to it is what keeps the session service running.
        controller = MediaController.Builder(this, SessionToken(this, ComponentName(this, PlaybackService::class.java))).buildAsync()
        (application as MusicApp).devices.looking(true)
        val app = application as MusicApp
        app.scope.launch { app.retry() }
    }

    @androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    override fun onStop() {
        (application as MusicApp).devices.looking(false)
        // Closed or not, what plays is kept, to be found as it was.
        (application as MusicApp).playback.keepSession()
        controller?.let(MediaController::releaseFuture)
        controller = null
        super.onStop()
    }

    /** Whether the app is dark now, for the bars' icons. */
    private var darkBars = true

    private fun barsAsApp() {
        androidx.core.view.WindowCompat.getInsetsController(window, window.decorView).run {
            isAppearanceLightStatusBars = !darkBars
            isAppearanceLightNavigationBars = !darkBars
        }
    }

    /** The system turned light or dark (the screen kept: see the manifest): the bars stay as the app. */
    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        super.onConfigurationChanged(newConfig)
        window.decorView.post { barsAsApp() }
    }

    /** In the app, the volume keys set the volume of the device played on, if any. */
    @androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    override fun onKeyDown(keyCode: Int, event: android.view.KeyEvent): Boolean {
        val up = keyCode == android.view.KeyEvent.KEYCODE_VOLUME_UP
        if ((up || keyCode == android.view.KeyEvent.KEYCODE_VOLUME_DOWN) && (application as MusicApp).playback.stepRemoteVolume(up)) return true
        return super.onKeyDown(keyCode, event)
    }

    /** The window as it shows, for a theme to fade from; null if it couldn't be copied. */
    private suspend fun snapshot(): android.graphics.Bitmap? {
        val view = window.decorView
        if (view.width == 0 || view.height == 0) return null
        val bitmap = android.graphics.Bitmap.createBitmap(view.width, view.height, android.graphics.Bitmap.Config.ARGB_8888)
        return kotlinx.coroutines.suspendCancellableCoroutine { done ->
            android.view.PixelCopy.request(window, bitmap, { result ->
                done.resumeWith(Result.success(bitmap.takeIf { result == android.view.PixelCopy.SUCCESS }))
            }, android.os.Handler(mainLooper))
        }
    }

    override fun onNewIntent(intent: android.content.Intent) {
        super.onNewIntent(intent)
        playOpened(intent)
    }

    /** A music file opened from another app (Files, a chat): it plays. */
    private fun playOpened(intent: android.content.Intent?) {
        val uri = intent?.data?.takeIf { intent.action == android.content.Intent.ACTION_VIEW } ?: return
        val app = application as MusicApp
        app.scope.launch {
            val track = app.localFiles.opened(uri)
            app.playback.play(listOf(track), 0)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            notifications.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
        val app = application as MusicApp
        playOpened(intent)
        setContent {
            val appearance by app.core.appearance.collectAsStateWithLifecycle()
            val themes by app.themes.all.collectAsStateWithLifecycle()
            val chosen by app.themes.chosen.collectAsStateWithLifecycle()
            val connecting = remember { Connecting() }
            var welcomed by remember { mutableStateOf(app.core.welcomed) }
            LaunchedEffect(Unit) { app.start() }
            // Another theme or appearance: the screen as it was, taken before
            // the new one is drawn, fades away over it.
            var applied by remember { mutableStateOf(chosen to appearance) }
            var veil by remember { mutableStateOf<androidx.compose.ui.graphics.ImageBitmap?>(null) }
            val fade = remember { androidx.compose.animation.core.Animatable(0f) }
            LaunchedEffect(chosen, appearance) {
                if (applied == chosen to appearance) return@LaunchedEffect
                veil = snapshot()?.asImageBitmap()
                fade.snapTo(1f)
                applied = chosen to appearance
                fade.animateTo(0f, androidx.compose.animation.core.tween(380))
                veil = null
            }
            MusicTheme(applied.second, app.themes.current(themes, applied.first)) {
                // The bars' icons as the app's appearance, not the system's:
                // dark on a light app, light on a dark one.
                val dark = io.github.honeyslatt.music.ui.theme.roles.dark
                androidx.compose.runtime.SideEffect {
                    darkBars = dark
                    barsAsApp()
                }
                Box(Modifier.fillMaxSize()) {
                    if (welcomed) {
                        App(app.core, app.library, app.playback, app.settings, connecting)
                    } else {
                        Welcome(app.core, connecting) {
                            app.core.welcomed = true
                            welcomed = true
                        }
                    }
                    ConnectHost(connecting, app.core)
                    val check by app.core.soundCloudWeb.check.collectAsStateWithLifecycle()
                    check?.let { done -> io.github.honeyslatt.music.ui.connect.SoundCloudCheck { done.complete(Unit) } }
                    veil?.let {
                        androidx.compose.foundation.Image(
                            it,
                            null,
                            Modifier.fillMaxSize().graphicsLayer { alpha = fade.value },
                            contentScale = androidx.compose.ui.layout.ContentScale.FillBounds,
                        )
                    }
                }
            }
        }
    }
}
