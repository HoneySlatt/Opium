package io.github.honeyslatt.music.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The sign-ins, encrypted with a key that Android Keystore holds and never
 * hands out: readable by Music only, on this phone only.
 */
class Vault(context: Context) {
    private val dir = File(context.filesDir, "vault").apply { mkdirs() }

    private fun key(): SecretKey {
        val store = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (store.getKey(ALIAS, null) as? SecretKey)?.let { return it }
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

    fun write(name: String, text: String) {
        val cipher = Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.ENCRYPT_MODE, key()) }
        val sealed = cipher.doFinal(text.toByteArray())
        File(dir, name).writeBytes(cipher.iv + sealed)
    }

    /** What was kept under `name`; nothing when it can't be read. */
    fun read(name: String): String? {
        val bytes = File(dir, name).takeIf { it.exists() }?.readBytes() ?: return null
        if (bytes.size <= IV) return null
        return runCatching {
            val cipher = Cipher.getInstance(TRANSFORMATION).apply {
                init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes, 0, IV))
            }
            String(cipher.doFinal(bytes, IV, bytes.size - IV))
        }.getOrNull()
    }

    fun has(name: String) = File(dir, name).exists()

    fun delete(name: String) {
        File(dir, name).delete()
    }

    private companion object {
        const val KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "music-vault"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val IV = 12
    }
}
