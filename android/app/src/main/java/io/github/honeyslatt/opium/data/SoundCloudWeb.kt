package io.github.honeyslatt.opium.data

import android.annotation.SuppressLint
import android.content.Context
import android.webkit.JavascriptInterface
import android.webkit.WebView
import android.webkit.WebViewClient
import io.github.honeyslatt.opium.core.SoundCloudAnswer
import io.github.honeyslatt.opium.core.SoundCloudBrowser
import kotlin.coroutines.resume
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONObject

/**
 * Sends SoundCloud's changes (likes, follows, playlists) from a WebView no
 * one sees, on soundcloud.com with its cookies: SoundCloud's bot guard takes
 * only a browser's. When the guard asks for a check, [check] is set for the
 * app to show SoundCloud, where the user passes it; then the change is sent
 * again.
 */
class SoundCloudWeb(private val context: Context) : SoundCloudBrowser {
    companion object {
        /**
         * The WebView's own Chrome, said to be on a computer: SoundCloud's
         * mobile site no longer keeps the token in a cookie, its desktop one
         * does; and the bot guard sees the same browser throughout.
         */
        fun desktopAgent(context: Context): String {
            val chrome = Regex("Chrome/[0-9.]+").find(android.webkit.WebSettings.getDefaultUserAgent(context))?.value ?: "Chrome/141.0.0.0"
            return "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) $chrome Safari/537.36"
        }
    }

    private val _check = MutableStateFlow<CompletableDeferred<Unit>?>(null)

    /** Completed by the app once the user is done with the check. */
    val check: StateFlow<CompletableDeferred<Unit>?> = _check.asStateFlow()

    /** One change at a time, and one check. */
    private val one = Mutex()

    override suspend fun send(method: String, url: String, headers: Map<String, String>, body: String?): SoundCloudAnswer = one.withLock {
        val answer = fetch(method, url, headers, body)
        if (!(answer.status.toInt() == 403 && "captcha-delivery.com" in answer.body)) return@withLock answer
        val done = CompletableDeferred<Unit>()
        _check.value = done
        try {
            // Not passed within ten minutes: the change is sent again, and refused if still asked.
            withTimeoutOrNull(600_000) { done.await() }
        } finally {
            _check.value = null
        }
        fetch(method, url, headers, body)
    }

    @SuppressLint("SetJavaScriptEnabled")
    private suspend fun fetch(method: String, url: String, headers: Map<String, String>, body: String?): SoundCloudAnswer =
        withContext(Dispatchers.Main) {
            withTimeoutOrNull(30_000) {
                suspendCancellableCoroutine { done ->
                    val web = WebView(context)
                    web.settings.javaScriptEnabled = true
                    web.settings.userAgentString = desktopAgent(context)
                    // The API is another host than the page's: its cookies, the guard's, go along.
                    android.webkit.CookieManager.getInstance().setAcceptThirdPartyCookies(web, true)
                    web.addJavascriptInterface(
                        object {
                            @JavascriptInterface
                            fun answer(status: Int, text: String) {
                                web.post { web.destroy() }
                                if (done.isActive) done.resume(SoundCloudAnswer(status.toUShort(), text))
                            }
                        },
                        "opium",
                    )
                    val script = """
                        (async () => {
                          const init = { method: ${JSONObject.quote(method)}, headers: ${JSONObject(headers)}, credentials: 'include' };
                          const body = ${body?.let(JSONObject::quote) ?: "null"};
                          if (body !== null) init.body = body;
                          try {
                            const r = await fetch(${JSONObject.quote(url)}, init);
                            opium.answer(r.status, await r.text());
                          } catch (e) {
                            opium.answer(0, String(e));
                          }
                        })();
                    """.trimIndent()
                    var sent = false
                    web.webViewClient = object : WebViewClient() {
                        override fun onPageFinished(view: WebView, page: String?) {
                            if (sent) return
                            sent = true
                            view.evaluateJavascript(script, null)
                        }
                    }
                    // An empty page on soundcloud.com: the request is the site's own.
                    web.loadDataWithBaseURL("https://soundcloud.com/", "<html></html>", "text/html", "utf-8", null)
                    done.invokeOnCancellation { web.post { web.destroy() } }
                }
            } ?: SoundCloudAnswer(0u, "no answer")
        }
}
