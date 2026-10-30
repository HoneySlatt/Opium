package io.github.honeyslatt.music.ui

import android.annotation.SuppressLint
import android.webkit.CookieManager
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.delay

/** Until the onboarding screens: a button to Apple's sign-in page. */
@Composable
fun SignInPrompt(onSignIn: () -> Unit) {
    Column(
        Modifier.fillMaxSize().background(roles.base).windowInsetsPadding(WindowInsets.safeDrawing).padding(32.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        BasicText("Music", style = geist(32.sp, FontWeight.Bold).copy(color = roles.text))
        val shape = RoundedCornerShape(14.dp)
        Box(
            Modifier.height(52.dp).press(shape, shrink = 0.97f, onClick = onSignIn).background(roles.accent, shape).padding(horizontal = 24.dp),
            contentAlignment = Alignment.Center,
        ) {
            BasicText("Sign in to Apple Music", style = geist(17.sp, FontWeight.SemiBold).copy(color = roles.onAccent))
        }
    }
}

/** Apple's sign-in page; done once its cookies hold the user's token. */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun AppleSignIn(done: (String) -> Unit) {
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
        modifier = Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing),
    )
}
