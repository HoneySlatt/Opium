package io.github.honeyslatt.opium.data

import java.io.File
import java.nio.file.Files
import java.security.KeyStoreException
import java.security.ProviderException
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.SecretKeySpec
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

/** How a read of the vault counts (audit finding L7), and what each case allows. */
class VaultReadTest {
    /** This test's own folder, under the system's temporary folder. */
    private val folder: File = Files.createTempDirectory("opium-vault-test").toFile()

    @AfterTest
    fun clean() {
        folder.deleteRecursively()
    }

    private fun newKey(): SecretKey = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()

    private val key = newKey()

    private fun kept(text: String, sealedWith: SecretKey = key) =
        File(folder, "kept").apply { writeBytes(seal(sealedWith, text)) }

    private val neverInvalidated: (Exception) -> Boolean = { false }

    /** Thrown by a test's Keystore for a key invalidated for good. */
    private class Invalidated : Exception()

    @Test
    fun `no file is missing`() {
        assertEquals(VaultRead.Missing, readSealed(File(folder, "none"), { key }, neverInvalidated))
    }

    @Test
    fun `a file sealed with the vault's key reads`() {
        assertEquals(VaultRead.Read("token"), readSealed(kept("token"), { key }, neverInvalidated))
        assertEquals(VaultRead.Read(""), readSealed(kept(""), { key }, neverInvalidated))
    }

    @Test
    fun `a file whose key the Keystore no longer holds is gone for good`() {
        assertIs<VaultRead.GoneForGood>(readSealed(kept("token"), { null }, neverInvalidated))
    }

    @Test
    fun `a file whose tag fails with a key present is gone for good`() {
        assertIs<VaultRead.GoneForGood>(readSealed(kept("token", sealedWith = newKey()), { key }, neverInvalidated))
        val damaged = kept("token").apply { writeBytes(readBytes().also { it[it.size - 1] = (it[it.size - 1] + 1).toByte() }) }
        assertIs<VaultRead.GoneForGood>(readSealed(damaged, { key }, neverInvalidated))
    }

    @Test
    fun `a file too short to hold a nonce and a tag is gone for good`() {
        for (size in listOf(0, 12, 27)) {
            val short = File(folder, "short-$size").apply { writeBytes(ByteArray(size)) }
            assertIs<VaultRead.GoneForGood>(readSealed(short, { key }, neverInvalidated), "$size bytes")
        }
    }

    @Test
    fun `a key invalidated for good is gone for good`() {
        val read = readSealed(kept("token"), { throw Invalidated() }, { it is Invalidated })
        assertIs<VaultRead.GoneForGood>(read)
    }

    @Test
    fun `a Keystore that fails is failed for now`() {
        for (failure in listOf(KeyStoreException("busy"), ProviderException("Keystore operation failed"))) {
            assertIs<VaultRead.FailedForNow>(readSealed(kept("token"), { throw failure }, neverInvalidated), "$failure")
        }
    }

    @Test
    fun `a cipher that refuses the key for another reason than its tag is failed for now`() {
        val refused = SecretKeySpec(ByteArray(5), "AES")
        assertIs<VaultRead.FailedForNow>(readSealed(kept("token"), { refused }, neverInvalidated))
    }

    @Test
    fun `a file that can't be read is failed for now, never thrown`() {
        val unreadable = File(folder, "a-folder").apply { mkdirs() }
        assertIs<VaultRead.FailedForNow>(readSealed(unreadable, { key }, neverInvalidated))
    }

    @Test
    fun `only a key gone for good sweeps the downloads, and only one missing or gone is made anew`() {
        assertEquals(DownloadsKeyStep.MakeNew(sweepFirst = false), downloadsKeyStep(VaultRead.Missing))
        assertEquals(DownloadsKeyStep.Use("key"), downloadsKeyStep(VaultRead.Read("key")))
        assertEquals(DownloadsKeyStep.MakeNew(sweepFirst = true), downloadsKeyStep(VaultRead.GoneForGood("no key")))
        assertEquals(DownloadsKeyStep.Unavailable("busy"), downloadsKeyStep(VaultRead.FailedForNow("busy")))
    }

    @Test
    fun `only a sign-in gone for good is dropped`() {
        assertEquals(SignInStep.NotSignedIn, signInStep(VaultRead.Missing))
        assertEquals(SignInStep.Use("token"), signInStep(VaultRead.Read("token")))
        assertEquals(SignInStep.SignInAgain, signInStep(VaultRead.GoneForGood("no key")))
        assertEquals(SignInStep.TryLater("busy"), signInStep(VaultRead.FailedForNow("busy")))
    }

    @Test
    fun `a secret read is never printed`() {
        assertEquals(false, "token" in VaultRead.Read("token").toString())
        assertEquals(false, "token" in DownloadsKeyStep.Use("token").toString())
        assertEquals(false, "token" in SignInStep.Use("token").toString())
    }
}
