package io.github.honeyslatt.opium.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import java.io.File
import java.io.IOException
import java.security.KeyStore
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey

/**
 * The sign-ins, encrypted with a key that Android Keystore holds and never
 * hands out: readable by Opium only, on this phone only.
 */
class Vault(context: Context) {
    private val dir = File(context.filesDir, "vault").apply { mkdirs() }

    private fun keyStore(): KeyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }

    /**
     * The vault's key, made when the Keystore holds none. Only a write makes one: a read that did
     * would hide a lost key behind a new one, which opens nothing kept.
     */
    private fun keyToSeal(): SecretKey {
        (keyStore().getKey(ALIAS, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE).apply {
            init(
                KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .build(),
            )
        }.generateKey()
    }

    /**
     * Keep `text` under `name`, whole or not at all: written aside, then put in place. A failure
     * (the Keystore failing, the disk full) is returned, never thrown, and leaves what was kept
     * before as it was. Should the cipher say the key was invalidated for good
     * (`KeyPermanentlyInvalidatedException`), a new key replaces it; Android's `getKey` reports
     * such a key as an `UnrecoverableKeyException` instead, which fails the write, as
     * [readSealed] says.
     */
    @Synchronized
    fun write(name: String, text: String): Result<Unit> = try {
        val sealed = try {
            seal(keyToSeal(), text)
        } catch (e: KeyPermanentlyInvalidatedException) {
            // It opens nothing any more: a new key seals from now on.
            keyStore().deleteEntry(ALIAS)
            seal(keyToSeal(), text)
        }
        val part = File(dir, "$name.part")
        part.outputStream().use { out ->
            out.write(sealed)
            out.fd.sync()
        }
        if (!part.renameTo(File(dir, name))) throw IOException("couldn't put $name in place")
        Result.success(Unit)
    } catch (e: Exception) {
        Result.failure(e)
    }

    /**
     * What is kept under `name`: missing, read, gone for good or failed for now ([VaultRead]
     * says what each allows). Never throws, and never makes a key.
     */
    fun read(name: String): VaultRead = readSealed(
        File(dir, name),
        key = { keyStore().getKey(ALIAS, null) as? SecretKey },
        invalidatedForGood = { it is KeyPermanentlyInvalidatedException },
    )

    fun has(name: String) = File(dir, name).exists()

    /**
     * What is kept under `name` deleted. A [write] under way ends first: deleted in its middle, the
     * file would come back with its rename.
     */
    @Synchronized
    fun delete(name: String) {
        File(dir, name).delete()
    }

    private companion object {
        const val KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "opium-vault"
    }
}
