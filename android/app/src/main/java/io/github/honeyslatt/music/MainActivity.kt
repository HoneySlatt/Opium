package io.github.honeyslatt.music

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.ui.App
import io.github.honeyslatt.music.ui.AppleSignIn
import io.github.honeyslatt.music.ui.SignInPrompt
import io.github.honeyslatt.music.ui.theme.MusicTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val app = application as MusicApp
        setContent {
            val appearance by app.core.appearance.collectAsStateWithLifecycle()
            val apple by app.core.apple.collectAsStateWithLifecycle()
            var signingIn by remember { mutableStateOf(false) }
            val scope = rememberCoroutineScope()
            LaunchedEffect(Unit) { app.core.start() }
            // The library loads once Apple Music answers.
            LaunchedEffect(apple is Connection.Connected) {
                if (apple is Connection.Connected) app.library.load(ProviderKind.APPLE_MUSIC)
            }
            MusicTheme(appearance) {
                when {
                    signingIn -> AppleSignIn { token ->
                        signingIn = false
                        scope.launch { app.core.signedInToApple(token) }
                    }
                    !app.core.hasAppleToken && apple !is Connection.Connecting -> SignInPrompt { signingIn = true }
                    else -> App(app.core, app.library)
                }
            }
        }
    }
}
