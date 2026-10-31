package io.github.honeyslatt.music.data

/**
 * A server's covers are kept and shown by an address without the sign-in;
 * only as one is fetched is it signed (see the core's `signed_cover`), so
 * that no other app, nor anything kept, sees it.
 */
object Covers {
    @Volatile
    var sign: (String) -> String = { it }

    /** Fetched through Coil: signed there, kept in its caches by the address as shown. */
    class Signing : coil3.intercept.Interceptor {
        override suspend fun intercept(chain: coil3.intercept.Interceptor.Chain): coil3.request.ImageResult {
            val shown = chain.request.data as? String ?: return chain.proceed()
            val from = sign(shown)
            if (from == shown) return chain.proceed()
            val signed = chain.request.newBuilder()
                .data(from)
                .memoryCacheKey(shown)
                .diskCacheKey(shown)
                .build()
            return chain.withRequest(signed).proceed()
        }
    }
}
