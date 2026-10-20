package me.kavishdevar.librepods.privileged

import android.annotation.SuppressLint
import android.app.ActivityThread
import android.bluetooth.BluetoothHeadset
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.content.Context
import android.os.Looper
import android.util.Log
import kotlin.system.exitProcess

object Main {
    @SuppressLint("MissingPermission")
    @JvmStatic
    fun main(args: Array<String>) {
        Log.i("LibrePodsPrivileged", "Starting")

        @Suppress("DEPRECATION")
        Looper.prepareMainLooper()

        ActivityThread.initializeMainlineModules()

        val context = ActivityThread.systemMain().systemContext as Context

        val adapter = context.getSystemService(BluetoothManager::class.java).adapter

        check(adapter != null) { "Bluetooth adapter unavailable" }
        check(adapter.isEnabled) { "Bluetooth adapter not enabled" }

        Log.i(
            "LibrePodsPrivileged",
            "Getting remote device"
        )

        val device = adapter.getRemoteDevice("30:7A:D2:42:77:40")

        Log.i(
            "LibrePodsPrivileged",
            "Got remote device: ${device.name} (${device.address})"
        )


        val started = adapter.getProfileProxy(
            context,
            object : BluetoothProfile.ServiceListener {
                override fun onServiceConnected(
                    profile: Int,
                    proxy: BluetoothProfile
                ) {
                    if (profile != BluetoothProfile.HEADSET) return

                    val headset = proxy as BluetoothHeadset

                    try {
                        val success = headset.setConnectionPolicy(
                            device,
                            BluetoothProfile.CONNECTION_POLICY_FORBIDDEN
                        )

                        Log.i(
                            "LibrePodsPrivileged",
                            "Set HFP forbidden: success=$success"
                        )
                    } catch (e: Exception) {
                        Log.e("LibrePodsPrivileged", "Failed to disable HFP", e)
                    } finally {
                        adapter.closeProfileProxy(profile, proxy)
                        exitProcess(0)
                    }
                }

                override fun onServiceDisconnected(profile: Int) {
                    Log.w("LibrePodsPrivileged", "Profile disconnected: $profile")
                }
            },
            BluetoothProfile.HEADSET
        )

        check(started) { "getProfileProxy() rejected the request" }

        Log.i("LibrePodsPrivileged", "Waiting for HFP proxy")
        Looper.loop()
    }
}
