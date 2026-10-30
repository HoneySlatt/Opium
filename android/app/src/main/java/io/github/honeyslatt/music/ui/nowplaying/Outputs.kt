package io.github.honeyslatt.music.ui.nowplaying

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.media.MediaRouter2
import androidx.compose.ui.graphics.vector.ImageVector
import io.github.honeyslatt.music.ui.Icons

enum class OutputKind { Speaker, Wired, Usb, Bluetooth, Other }

/** Where the sound goes: the device's name, an icon and its kind. */
data class OutputDevice(val name: String, val icon: ImageVector, val kind: OutputKind)

object Outputs {
    private val media = AudioAttributes.Builder()
        .setUsage(AudioAttributes.USAGE_MEDIA)
        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
        .build()

    /** The device media plays on now, as Android routes it. */
    fun current(context: Context): OutputDevice {
        val audio = context.getSystemService(AudioManager::class.java)
        val device = audio.getAudioDevicesForAttributes(media).firstOrNull()
        return when (device?.type) {
            null, AudioDeviceInfo.TYPE_BUILTIN_SPEAKER, AudioDeviceInfo.TYPE_BUILTIN_EARPIECE ->
                OutputDevice("This Phone", Icons.Phone, OutputKind.Speaker)
            AudioDeviceInfo.TYPE_USB_HEADSET, AudioDeviceInfo.TYPE_USB_DEVICE ->
                OutputDevice(device.productName?.toString()?.takeIf { it.isNotBlank() } ?: "USB", Icons.Usb, OutputKind.Usb)
            AudioDeviceInfo.TYPE_WIRED_HEADPHONES, AudioDeviceInfo.TYPE_WIRED_HEADSET ->
                OutputDevice("Headphones", Icons.Headphones, OutputKind.Wired)
            AudioDeviceInfo.TYPE_BLUETOOTH_A2DP, AudioDeviceInfo.TYPE_BLE_HEADSET,
            AudioDeviceInfo.TYPE_BLE_SPEAKER, AudioDeviceInfo.TYPE_BLE_BROADCAST ->
                OutputDevice(device.productName?.toString()?.takeIf { it.isNotBlank() } ?: "Bluetooth", Icons.Headphones, OutputKind.Bluetooth)
            else -> OutputDevice(device.productName?.toString()?.takeIf { it.isNotBlank() } ?: "Output", Icons.Phone, OutputKind.Other)
        }
    }

    /** The rate Android's mixer runs at, in Hz. */
    fun mixerRate(context: Context): Int? =
        context.getSystemService(AudioManager::class.java)
            .getProperty(AudioManager.PROPERTY_OUTPUT_SAMPLE_RATE)?.toIntOrNull()

    /** The media volume, from 0 to 1. */
    fun volume(context: Context): Float {
        val audio = context.getSystemService(AudioManager::class.java)
        val max = audio.getStreamMaxVolume(AudioManager.STREAM_MUSIC).coerceAtLeast(1)
        return audio.getStreamVolume(AudioManager.STREAM_MUSIC).toFloat() / max
    }

    /** Android's own output switcher. */
    fun showSwitcher(context: Context) {
        MediaRouter2.getInstance(context).showSystemOutputSwitcher()
    }
}
