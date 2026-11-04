package io.github.honeyslatt.opium.core

import android.content.Context

/** The Rust core's TLS checks certificates against Android's own store. */
object Tls {
    init {
        System.loadLibrary("opium_ffi")
    }

    /** Once, before the core makes any request. */
    @JvmStatic
    external fun init(context: Context)
}
