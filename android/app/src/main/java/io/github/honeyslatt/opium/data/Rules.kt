package io.github.honeyslatt.opium.data

// The data layer's decisions that need no Android: how a read of the vault counts and what each
// case allows, where the desktop's themes go beside the user's own, and the order in which the
// downloads' store is told of the services. Plain Kotlin and Java only, so that the JVM tests of
// `android/app/src/test/` check them; `Vault`, `Downloads`, `Core` and `Themes` call them.

import java.io.File
import java.nio.file.Files
import java.nio.file.NoSuchFileException
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** What reading a secret the vault keeps found: one of four cases, which allow different things. */
sealed interface VaultRead {
    /** Nothing is kept under that name. */
    data object Missing : VaultRead

    /** What was kept. */
    data class Read(val text: String) : VaultRead {
        // A secret: never printed.
        override fun toString() = "Read(…)"
    }

    /**
     * Kept, but never readable again: the Keystore holds no key for the vault any more (as after
     * a restore onto another phone), Android invalidated the key for good, or the file does not
     * open with the key (its tag fails: sealed under another key, or damaged). What it held is
     * lost: the only case in which what depends on it may be deleted, a sign-in dropped, or a new
     * key made in place of one kept.
     */
    data class GoneForGood(val why: String) : VaultRead

    /**
     * Kept, but not readable this time: the Keystore failing (`KeyStoreException`,
     * `ProviderException`…) or the file not read. Everything is left as it is, and the next
     * try, at the next start at the latest, reads it again.
     */
    data class FailedForNow(val why: String) : VaultRead
}

/** The vault's sealing: AES-GCM, the 12-byte nonce first, then the sealed text and its 16-byte tag. */
private const val TRANSFORMATION = "AES/GCM/NoPadding"
private const val NONCE_BYTES = 12
private const val TAG_BITS = 128

/**
 * `text` sealed with `key`, as [readSealed] reads it: the nonce the cipher chose, then the sealed
 * text and its tag.
 *
 * @throws java.security.GeneralSecurityException or another exception from the cipher when it
 *   refuses the key: on Android, the Keystore failing, or `KeyPermanentlyInvalidatedException`.
 */
fun seal(key: SecretKey, text: String): ByteArray {
    val cipher = Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.ENCRYPT_MODE, key) }
    val sealed = cipher.doFinal(text.toByteArray())
    return cipher.iv + sealed
}

/**
 * The secret kept in `file`, sealed by [seal] with the vault's key, as one of the four cases of
 * [VaultRead]. Never throws: every exception is one of the cases.
 *
 * - Missing: no file (and only then: a file there that can't be read has failed for now).
 * - Gone for good: a file too short to hold a nonce and a tag (damaged); `key` giving none (the
 *   Keystore holds no key under the vault's alias: Android's `KeyStore.getKey` returns null only
 *   then); a tag that fails (`AEADBadTagException`, which Android's Keystore throws only for
 *   `KM_ERROR_VERIFICATION_FAILED`); an exception that `invalidatedForGood` says means the key was
 *   invalidated for good (`KeyPermanentlyInvalidatedException`: an Android type, so asked for).
 * - Failed for now: any other exception, from reading the file, from `key` (the Keystore not
 *   loading, `UnrecoverableKeyException`…) or from the cipher (`ProviderException`, an
 *   `InvalidKeyException` or `IllegalBlockSizeException` from a Keystore error…).
 *
 * A key invalidated for good counts as gone for good only when the cipher says so: Android's
 * `KeyStore.getKey` reports such a key as an `UnrecoverableKeyException`, without its cause
 * (`AndroidKeyStoreSpi.engineGetKey`), which can't be told from a passing failure and so counts
 * as failed for now. That fails safe: a vault in that state stays unreadable, and nothing is
 * deleted. Neither should happen: Android invalidates only keys that need the user to
 * authenticate, and the vault's key doesn't.
 */
fun readSealed(file: File, key: () -> SecretKey?, invalidatedForGood: (Exception) -> Boolean): VaultRead {
    val bytes = try {
        Files.readAllBytes(file.toPath())
    } catch (e: NoSuchFileException) {
        return VaultRead.Missing
    } catch (e: Exception) {
        return VaultRead.FailedForNow("couldn't read its file (${e.describe()})")
    }
    if (bytes.size < NONCE_BYTES + TAG_BITS / 8) return VaultRead.GoneForGood("its file is damaged")
    return try {
        val key = key() ?: return VaultRead.GoneForGood("the Keystore holds no key for it")
        val cipher = Cipher.getInstance(TRANSFORMATION).apply {
            init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, bytes, 0, NONCE_BYTES))
        }
        VaultRead.Read(String(cipher.doFinal(bytes, NONCE_BYTES, bytes.size - NONCE_BYTES)))
    } catch (e: AEADBadTagException) {
        VaultRead.GoneForGood("it doesn't open with the Keystore's key")
    } catch (e: Exception) {
        if (invalidatedForGood(e)) {
            VaultRead.GoneForGood("Android invalidated its key")
        } else {
            VaultRead.FailedForNow(e.describe())
        }
    }
}

/** An exception in a few words, for a reason shown or reported: its kind, and its message. */
private fun Exception.describe() = listOfNotNull(javaClass.simpleName, message).joinToString(": ")

/** What the downloads do with the key that seals them, from its read in the vault. */
sealed interface DownloadsKeyStep {
    /** Kept and read: the store opens with it. */
    data class Use(val kept: String) : DownloadsKeyStep {
        // A secret: never printed.
        override fun toString() = "Use(…)"
    }

    /**
     * None kept, or the one kept gone for good: a new key is made and kept. With `sweepFirst`,
     * the songs the lost key sealed are deleted first, as they can never be read again.
     */
    data class MakeNew(val sweepFirst: Boolean) : DownloadsKeyStep

    /**
     * Kept, but not readable now: the downloads are unavailable until the next start; nothing is
     * deleted, and no key is made.
     */
    data class Unavailable(val why: String) : DownloadsKeyStep
}

/**
 * The step `read` allows the downloads' key: only a key gone for good sweeps the downloads, and
 * only a key missing or gone for good is made anew.
 */
fun downloadsKeyStep(read: VaultRead): DownloadsKeyStep = when (read) {
    VaultRead.Missing -> DownloadsKeyStep.MakeNew(sweepFirst = false)
    is VaultRead.Read -> DownloadsKeyStep.Use(read.text)
    is VaultRead.GoneForGood -> DownloadsKeyStep.MakeNew(sweepFirst = true)
    is VaultRead.FailedForNow -> DownloadsKeyStep.Unavailable(read.why)
}

/** What signing in again with a sign-in the vault keeps does, from its read. */
sealed interface SignInStep {
    /** None kept: not signed in. */
    data object NotSignedIn : SignInStep

    /** Kept and read: signed in with it. */
    data class Use(val kept: String) : SignInStep {
        // A secret: never printed.
        override fun toString() = "Use(…)"
    }

    /** Gone for good: the sign-in is deleted, and the user asked to sign in again. */
    data object SignInAgain : SignInStep

    /** Not readable now: the sign-in stays kept, shows as failed, and is tried again later. */
    data class TryLater(val why: String) : SignInStep
}

/** The step `read` allows a kept sign-in: only one gone for good is dropped. */
fun signInStep(read: VaultRead): SignInStep = when (read) {
    VaultRead.Missing -> SignInStep.NotSignedIn
    is VaultRead.Read -> SignInStep.Use(read.text)
    is VaultRead.GoneForGood -> SignInStep.SignInAgain
    is VaultRead.FailedForNow -> SignInStep.TryLater(read.why)
}

/** What the phone's settings keep of the desktop's themes. */
data class FromDesktop(
    /** The ids of the theme files that hold the desktop's themes; every other is the user's own. */
    val ids: Set<String>,
    /** By the desktop's id (made safe), the id its theme is kept under on the phone. */
    val places: Map<String, String>,
)

/**
 * Keep the desktop's themes as it shares them now, all of them, beside the user's own; returns
 * what the settings keep of them now.
 *
 * - `shared`: each theme's id (a file's stem, made safe) and its file's text, in the desktop's order;
 * - `held`: the ids of the theme files the phone holds;
 * - `before`: what the settings kept of the desktop's themes;
 * - `keep` writes a theme's file under an id, in place of the one there, and says whether it did
 *   (a file that isn't a theme, or a write that fails, isn't kept);
 * - `remove` takes the theme of an id away;
 * - `save` keeps a [FromDesktop] in the settings, and says whether it did.
 *
 * A theme of the user's is never replaced nor removed: a desktop theme of its id is kept under
 * another id ([desktopThemeIds]). The desktop's themes it no longer shares, or that no longer
 * keep, are removed; only those.
 *
 * The ids about to hold the desktop's themes are saved before any file is written, so that a
 * file written is never taken for the user's, even if the app stops midway; if they can't be
 * saved, nothing is written, and `before` is returned. Saved again at the end, without what went;
 * should that save fail, the settings keep more ids than hold the desktop's themes, none of them
 * the user's, which harms nothing.
 */
fun shareDesktopThemes(
    shared: List<Pair<String, String>>,
    held: Set<String>,
    before: FromDesktop,
    keep: (id: String, source: String) -> Boolean,
    remove: (id: String) -> Unit,
    save: (FromDesktop) -> Boolean,
): FromDesktop {
    val desktopIds = shared.map { it.first }
    val ids = desktopThemeIds(desktopIds, usersOwn = held - before.ids, placed = before.places)
    if (!save(FromDesktop(before.ids + ids, before.places + placesOf(desktopIds.zip(ids))))) return before
    val kept = shared.indices.filter { keep(ids[it], shared[it].second) }
    val now = FromDesktop(kept.map { ids[it] }.toSet(), placesOf(kept.map { desktopIds[it] to ids[it] }))
    (before.ids + ids - now.ids).forEach(remove)
    save(now)
    return now
}

/** By desktop id, its theme's id on the phone; of two themes of one desktop id, the first's. */
private fun placesOf(themes: List<Pair<String, String>>): Map<String, String> = themes.distinctBy { it.first }.toMap()

/**
 * The ids the desktop's themes take on the phone, one for each of `shared` (their ids, made
 * safe), in its order.
 *
 * A theme keeps the id it was kept under before (`placed`, by its desktop id), so that the user's
 * choice of it holds, unless one of the user's own themes holds that id now (`usersOwn`). Else it
 * takes its own id, unless one of the user's themes or another of the desktop's took it: then the
 * first free of `<id>-desktop`, `<id>-desktop-2`, `<id>-desktop-3`… Every theme that can take its
 * own id does so before any takes another, so that a desktop theme named `<id>-desktop` keeps its
 * name. The same list and places give the same ids.
 */
fun desktopThemeIds(shared: List<String>, usersOwn: Set<String>, placed: Map<String, String> = emptyMap()): List<String> {
    val taken = usersOwn.toMutableSet()
    val ids = MutableList<String?>(shared.size) { null }
    for ((i, id) in shared.withIndex()) {
        val before = placed[id] ?: continue
        if (taken.add(before)) ids[i] = before
    }
    for ((i, id) in shared.withIndex()) {
        if (ids[i] == null && taken.add(id)) ids[i] = id
    }
    for ((i, id) in shared.withIndex()) {
        if (ids[i] != null) continue
        var other = "$id-desktop"
        var n = 1
        while (other in taken) other = "$id-desktop-${++n}"
        taken += other
        ids[i] = other
    }
    return ids.requireNoNulls()
}

/**
 * What the phone tells its downloads' store of the services, in the order it was said.
 *
 * The store opens a moment after the app starts. What is said meanwhile waits, in order, and is
 * told once it opens, as the Rust queue keeps the confirmations said while its own store opens
 * (`StoreState::Opening` in `crates/downloads/src/queue.rs`); if it can't open, all is dropped
 * with it. Once open, what is said is told at once.
 *
 * A confirmation of a subscription is said when it is asked for, though its answer (Apple's) comes
 * later: a sign-out said meanwhile comes after it, and so wins. Its answer, come after the
 * sign-out, is dropped, which leaves the store as if told before the sign-out.
 *
 * As it opens, the store is also told a sign-out of every service not signed in then: a sign-out
 * said while it was closed, in a start whose store couldn't open, is thus never lost.
 *
 * `S` is the store, `P` a service; `forget` tells a store that a service was signed out of.
 * Safe from any thread. An order runs under this object's lock: it must not call back into it.
 */
class StoreOrders<S : Any, P : Any>(private val forget: (S, P) -> Unit) {
    private var store: S? = null
    private var opening = true
    private val waiting = mutableListOf<(S) -> Unit>()

    /** By service, how many times it was signed out of. */
    private val signOuts = mutableMapOf<P, Int>()

    /** The store, once open; none while it opens, or if it couldn't. */
    val open: S?
        @Synchronized get() = store

    /**
     * The store opened, or none if it couldn't be: what was said meanwhile is told it, in order,
     * then a sign-out of each of `notSignedIn`, the services not signed in now. Called once. An
     * exception from an order goes to the caller; the orders after it are dropped.
     */
    @Synchronized
    fun opened(store: S?, notSignedIn: Collection<P>) {
        notSignedIn.forEach(::signedOut)
        opening = false
        this.store = store
        val said = waiting.toList()
        waiting.clear()
        if (store != null) said.forEach { it(store) }
    }

    /** `provider`'s subscription asked about now: its answer, once known, goes to the [Confirmation]. */
    @Synchronized
    fun confirming(provider: P): Confirmation = Confirmation(provider, signOuts[provider] ?: 0)

    /** Signed out of `provider`: the store told so, after everything said before. */
    @Synchronized
    fun signedOut(provider: P) {
        signOuts[provider] = (signOuts[provider] ?: 0) + 1
        say { forget(it, provider) }
    }

    private fun say(order: (S) -> Unit) {
        val store = store
        when {
            store != null -> order(store)
            opening -> waiting += order
            // It couldn't open: there is nothing to tell.
        }
    }

    /** A confirmation of `provider`'s subscription, asked after `signOutsBefore` sign-outs of it. */
    inner class Confirmation internal constructor(private val provider: P, private val signOutsBefore: Int) {
        /** Its answer: `order` said, unless `provider` was signed out of since it was asked. */
        fun answered(order: (S) -> Unit) = synchronized(this@StoreOrders) {
            if ((signOuts[provider] ?: 0) == signOutsBefore) say(order)
        }
    }
}
