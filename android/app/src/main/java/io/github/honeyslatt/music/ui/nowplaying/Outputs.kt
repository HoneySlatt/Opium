package io.github.honeyslatt.music.ui.nowplaying

import android.content.Context
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.media.MediaRouter2
import androidx.compose.ui.graphics.vector.ImageVector
import io.github.honeyslatt.music.ui.Icons

/** Where the sound goes: the device's name and an icon for it. */
data class OutputDevice(val name: String, val icon: ImageVector)

object Outputs {
    /** The device media plays on now, as Android routes it. */
    fun current(context: Context): OutputDevice {
        val audio = context.getSystemService(AudioManager::class.java)
        val devices = audio.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
        // Media goes to what was plugged or paired last over the speaker.
        val order = listOf(
            AudioDeviceInfo.TYPE_USB_HEADSET,
            AudioDeviceInfo.TYPE_USB_DEVICE,
            AudioDeviceInfo.TYPE_BLE_HEADSET,
            AudioDeviceInfo.TYPE_BLE_SPEAKER,
            AudioDeviceInfo.TYPE_BLUETOOTH_A2DP,
            AudioDeviceInfo.TYPE_WIRED_HEADPHONES,
            AudioDeviceInfo.TYPE_WIRED_HEADSET,
        )
        val device = order.firstNotNullOfOrNull { type -> devices.firstOrNull { it.type == type } }
        return when (device?.type) {
            null -> OutputDevice("This Phone", Icons.Phone)
            AudioDeviceInfo.TYPE_USB_HEADSET, AudioDeviceInfo.TYPE_USB_DEVICE ->
                OutputDevice(device.productName?.toString() ?: "USB", Icons.Usb)
            AudioDeviceInfo.TYPE_WIRED_HEADPHONES, AudioDeviceInfo.TYPE_WIRED_HEADSET ->
                OutputDevice("Headphones", Icons.Headphones)
            else -> OutputDevice(device.productName?.toString() ?: "Bluetooth", Icons.Headphones)
        }
    }

    /** Android's own output switcher. */
    fun showSwitcher(context: Context) {
        MediaRouter2.getInstance(context).showSystemOutputSwitcher()
    }
}
