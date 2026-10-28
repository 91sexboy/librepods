package me.kavishdevar.librepods.core.bluetooth.ble

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.os.SystemClock
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import me.kavishdevar.librepods.core.bluetooth.MacAddress
import me.kavishdevar.librepods.core.bluetooth.verifyRPA
import me.kavishdevar.librepods.core.data.apple.AccessoryState
import me.kavishdevar.librepods.core.data.apple.DeviceColor
import me.kavishdevar.librepods.core.devices.AppleDevice
import me.kavishdevar.librepods.core.devices.Battery
import me.kavishdevar.librepods.core.devices.BatteryComponent
import me.kavishdevar.librepods.core.devices.BatterySource
import me.kavishdevar.librepods.core.devices.BatteryStatus
import me.kavishdevar.librepods.core.devices.ComponentStatus
import me.kavishdevar.librepods.core.devices.ConnectionState
import me.kavishdevar.librepods.core.devices.Device
import me.kavishdevar.librepods.core.devices.DeviceComponent
import me.kavishdevar.librepods.core.devices.DeviceComponentState
import me.kavishdevar.librepods.core.devices.format
import me.kavishdevar.librepods.core.devices.hasSameState
import javax.crypto.Cipher
import javax.crypto.spec.SecretKeySpec
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds

@SuppressLint("MissingPermission")
class BLEManager(
    private val context: Context,
    private val scope: CoroutineScope,

    private var scanMode: Int,
    private var reportDelay: Long,

    private val getDevices: () -> Map<MacAddress, Device<*, *, *>>,

    private var irkMap: Map<MacAddress, ByteArray>,
    private var encKeyMap: Map<MacAddress, ByteArray>,
) {
    companion object {
        private const val TAG = "BLEManager"
    }

    private val bluetoothAdapter: BluetoothAdapter? by lazy {
        context.getSystemService(BluetoothManager::class.java)?.adapter
    }

    private val rpasByPublicMac = mutableMapOf<MacAddress, MutableSet<MacAddress>>()
    private val caseAddressByPublicMac = mutableMapOf<MacAddress, MutableSet<MacAddress>>()
    private val rejectedRandomMac = mutableSetOf<MacAddress>()

    private var flushJob: Job? = null

    private val lastSeenMap = mutableMapOf<MacAddress, Long>()
    private var connectionTimeoutJob: Job? = null

    init {
        startTimeoutChecker()
    }

    private val timeoutDuration = 5.seconds

    private fun startTimeoutChecker() {
        connectionTimeoutJob = scope.launch {
            while (isActive) {
                delay(timeoutDuration / 2)
                val currentTime = System.currentTimeMillis()

                getDevices().values.forEach { device ->
                    val lastSeen = lastSeenMap[device.macAddress] ?: 0L
                    if (lastSeen > 0 && currentTime - lastSeen > timeoutDuration.inWholeMilliseconds) {
                        when (device) {
                            is AppleDevice -> {
                                if (device.connectionState.value == ConnectionState.AVAILABLE) {
                                    Log.i(
                                        TAG,
                                        "Device ${device.macAddress.toRedactedString()} timed out (no packets received). setting to DISCONNECTED"
                                    )
                                    device.updateConnectionState(ConnectionState.DISCONNECTED)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            processScanResult(result)
        }

        override fun onBatchScanResults(results: List<ScanResult?>?) {
            results?.filterNotNull()?.forEach { processScanResult(it) }
        }
    }

    fun start() {
        val bleScanner = bluetoothAdapter?.bluetoothLeScanner
        if (bleScanner == null) {
            Log.w(TAG, "error starting BLE scanner: not available")
            return
        }

        val scanSettings = ScanSettings.Builder()
            .setScanMode(scanMode)
            .setMatchMode(ScanSettings.MATCH_MODE_AGGRESSIVE)
            .setCallbackType(ScanSettings.CALLBACK_TYPE_ALL_MATCHES)
            .setNumOfMatches(ScanSettings.MATCH_NUM_MAX_ADVERTISEMENT)
            .setReportDelay(reportDelay)
            .build()

        val budsFilter = ScanFilter.Builder()
            .setManufacturerData(
                0x004C,
                byteArrayOf(0x07, 0x00, 0x01),
                byteArrayOf(0xFF.toByte(), 0x00, 0xFF.toByte())
            )
            .build()

        val chargingCaseFilter = ScanFilter.Builder()
            .setManufacturerData(
                0x004C,
                byteArrayOf(0x07, 0x00, 0x06),
                byteArrayOf(0xFF.toByte(), 0x00, 0xFF.toByte())
            )
            .build()

        try {
            bleScanner.startScan(listOf(budsFilter, chargingCaseFilter), scanSettings, scanCallback)
            Log.i(TAG, "started BLE scan: mode=${scanMode}, delay=${reportDelay}")
        } catch (e: Exception) {
            if (e.message?.contains("too frequently") == true) {
                scope.launch {
                    delay(5.seconds)
                    start()
                }
            } else {
                Log.e(TAG, "error starting BLE scan", e)
            }
        }

        flushJob = scope.launch {
            while (isActive) {
                delay(reportDelay.milliseconds)
                try {
                    bleScanner.flushPendingScanResults(scanCallback)
                } catch (e: Exception) {
                    Log.e(TAG, "error flushing pending scan results", e)
                }
            }
        }
    }

    fun stop() {
        flushJob?.cancel()
        connectionTimeoutJob?.cancel()
        val bleScanner = bluetoothAdapter?.bluetoothLeScanner ?: return
        try {
            bleScanner.stopScan(scanCallback)
            Log.i(TAG, "stopped BLE scan")
        } catch (e: Exception) {
            Log.e(TAG, "error stopping BLE scan", e)
        }
    }

    fun updateIrkMap(irkMap: Map<MacAddress, ByteArray>) {
        this.irkMap = irkMap
        rejectedRandomMac.clear()
    }

    fun updateEncKeyMap(encKeyMap: Map<MacAddress, ByteArray>) {
        this.encKeyMap = encKeyMap
        rejectedRandomMac.clear()
    }

    fun updateScanMode(scanMode: Int) {
        stop()
        this.scanMode = scanMode
        start()
    }

    fun updateReportDelay(reportDelay: Long) {
        stop()
        this.reportDelay = reportDelay
        start()
    }

    private fun processScanResult(result: ScanResult) {
        val macAddress = MacAddress(result.device.address)

        if (rejectedRandomMac.contains(macAddress)) return

        val manufacturerData = result.scanRecord?.getManufacturerSpecificData(0x004C)?: return


        if (handleAppleCaseAdvertisement(macAddress, manufacturerData)) {
            return
        }

        val deviceMac = resolveDeviceMac(macAddress)
        if (deviceMac != null) {
            handleResolvedAppleBudsAdvertisement(deviceMac, manufacturerData)
            return
        }

        rejectedRandomMac.add(macAddress)
    }

    private fun resolveDeviceMac(bleMac: MacAddress): MacAddress? {
        val devices = getDevices()
        var deviceMac: MacAddress? = devices[bleMac]?.macAddress

        if (deviceMac == null) {
            rpasByPublicMac.forEach { (address, addresses) ->
                if (addresses.contains(bleMac)) deviceMac = address
            }
        }

        if (deviceMac == null) {
            irkMap.forEach { (macAddress, irk) ->
                if (verifyRPA(bleMac.value, irk)) {
                    Log.i(TAG, "new RPA verified for device ${macAddress.toRedactedString()}")
                    deviceMac = macAddress
                    rpasByPublicMac.getOrPut(macAddress) { mutableSetOf() }.add(bleMac)
                }
            }
        }

        if (deviceMac == null) {
            rejectedRandomMac.add(bleMac)
        }

        return devices[deviceMac]?.macAddress
    }

    @Suppress("unused", "UnusedVariable")
    private fun handleResolvedAppleBudsAdvertisement(
        publicMac: MacAddress,
        data: ByteArray
    ) {
        lastSeenMap[publicMac] = System.currentTimeMillis()
        val encrypted = data.copyOfRange(data.size - 16, data.size)

        val plaintext = try {
            @SuppressLint("GetInstance")
            val cipher = Cipher.getInstance("AES/ECB/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(encKeyMap[publicMac], "AES"))
            cipher.doFinal(encrypted)
        } catch (_: Exception) {
            Log.e(TAG, "error decrypting data for device ${publicMac.toRedactedString()}")
            return
        }

        if (plaintext == null) {
            Log.e(TAG, "decrypted data is null for device ${publicMac.toRedactedString()}")
        }

        val paired = data[2].toInt() == 1
        val modelId = ((data[3].toInt() and 0xFF) shl 8) or (data[4].toInt() and 0xFF)

        val status = data[5].toInt() and 0xFF
        val lid = data[8].toInt() and 0xFF
        val color = DeviceColor.fromProximity(data[9])
        val state = AccessoryState.fromProximity(data[10])

        val primaryLeft = ((status shr 5) and 0x01) == 1
        val thisInCase = ((status shr 6) and 0x01) == 1
        val xorFactor = primaryLeft xor thisInCase

        val isLeftInEar = if (xorFactor) (status and 0x08) != 0 else (status and 0x02) != 0
        val isRightInEar = if (xorFactor) (status and 0x02) != 0 else (status and 0x08) != 0

        val isFlipped = !primaryLeft

        val leftByteIndex = if (isFlipped) 2 else 1
        val rightByteIndex = if (isFlipped) 1 else 2

        val batteries = buildSet {
            for ((component, index) in listOf(
                BatteryComponent.CASE to 3,
                BatteryComponent.LEFT to leftByteIndex,
                BatteryComponent.RIGHT to rightByteIndex
            )) {
                val raw = plaintext[index].toInt() and 0xFF
                if (raw == 0xFF) continue
                val charging = raw and 0x80 != 0
                add(
                    Battery(
                        component = component,
                        level = raw and 0x7F,
                        status = if (charging) BatteryStatus.CHARGING else BatteryStatus.NOT_CHARGING,
                        source = BatterySource.BUDS_ADVERT,
                        timestamp = System.currentTimeMillis()
                    )
                )
            }
        }

        val caseCanBeOpen = (!isLeftInEar || !isRightInEar) && batteries.any { it.status == BatteryStatus.CHARGING || it.status == BatteryStatus.OPTIMIZED_CHARGING }
        val lidOpen = ((lid shr 3) and 0x01) == 0 && caseCanBeOpen

        val device = (getDevices()[publicMac] as? AppleDevice)

        if (device != null) {
            val previousState = device.state.value

            val updates = batteries.associateBy { it.component }
            val merged = (previousState.battery.filterNot { it.component in updates } + updates.values).toSet()

            val budInCase = batteries.find { it.component == BatteryComponent.LEFT }?.status != BatteryStatus.CHARGING ||
                    batteries.find { it.component == BatteryComponent.LEFT }?.status != BatteryStatus.OPTIMIZED_CHARGING ||
                    batteries.find { it.component == BatteryComponent.RIGHT }?.status != BatteryStatus.CHARGING ||
                    batteries.find { it.component == BatteryComponent.RIGHT }?.status != BatteryStatus.OPTIMIZED_CHARGING

            device.updateState { state ->
                state.copy(
                    battery = merged
                )
            }

            if (!previousState.battery.hasSameState(merged)) {
                Log.d(
                    TAG,
                    "battery changed for ${device.macAddress.toRedactedString()}: ${device.state.value.battery.format()}"
                )
            }

            if ((previousState.primaryComponent != if (primaryLeft) DeviceComponent.LEFT else DeviceComponent.RIGHT) && !budInCase) {
                device.updateState { state ->
                    state.copy(
                        primaryComponent = if (primaryLeft) DeviceComponent.LEFT else DeviceComponent.RIGHT
                    )
                }
                Log.d(TAG, "primary component changed for ${device.macAddress.toRedactedString()}: ${if (primaryLeft) DeviceComponent.LEFT else DeviceComponent.RIGHT}")
            }

            if (previousState.state != state) {
                device.updateState { appleState ->
                    appleState.copy(
                        state = state
                    )
                }
                Log.d(TAG, "state changed for ${device.macAddress.toRedactedString()}: $state")
            }

            if (previousState.caseLidOpen != lidOpen) {
                device.updateState {
                    it.copy(
                        caseLidOpen = lidOpen
                    )
                }
                Log.d(TAG, "case lid open changed for ${device.macAddress.toRedactedString()}: $lidOpen")
            }

            if (device.metadata.value.color != color) {
                device.updateMetadata {
                    it.copy(
                        color = color
                    )
                }
            }

            if (device.connectionState.value == ConnectionState.DISCONNECTED) {
                device.updateConnectionState(ConnectionState.AVAILABLE)
            }

            if (device.connectionState.value != ConnectionState.CONNECTED) {
                val leftCharging = batteries.any { it.component == BatteryComponent.LEFT && it.status == BatteryStatus.CHARGING }
                val rightCharging = batteries.any { it.component == BatteryComponent.RIGHT && it.status == BatteryStatus.CHARGING }

                device.updateState {
                    it.copy(
                        componentState = setOf(
                            DeviceComponentState(
                                component = DeviceComponent.LEFT,
                                status = if (isLeftInEar) ComponentStatus.IN_EAR else if (leftCharging) ComponentStatus.IN_CASE else ComponentStatus.OUT_OF_EAR
                            ),
                            DeviceComponentState(
                                component = DeviceComponent.RIGHT,
                                status = if (isRightInEar) ComponentStatus.IN_EAR else if (rightCharging) ComponentStatus.IN_CASE else ComponentStatus.OUT_OF_EAR
                            )
                        )
                    )
                }
            }
        } else {
            Log.w(
                TAG,
                "that's weird! (device is not apple but has magic keys?)"
            )
            Log.d(
                TAG,
                "received advertisement from ${publicMac.toRedactedString()}: " +
                    "lidOpen=$lidOpen, state=${if (state == AccessoryState.Unknown) data[10].toHexString() else state}, " +
                    "isLeftInEar=$isLeftInEar, isRightInEar=$isRightInEar" + ", batteries=$batteries"
            )
        }
    }

    private fun handleAppleCaseAdvertisement(caseMac: MacAddress, data: ByteArray): Boolean {
        var offset = 0
        while (offset + 2 <= data.size) {
            val kind = data[offset].toInt() and 0xFF
            val rawLength = data[offset + 1].toInt() and 0xFF
            val length = rawLength and 0x1F
            val payloadStart = offset + 2
            val payloadEnd = payloadStart + length

            if (payloadEnd > data.size) break

            if (kind == 0x07 && length >= 17 && data[payloadStart] == 0x06.toByte()) {
                val encrypted = data.copyOfRange(payloadStart + 1, payloadStart + 17)

                for ((publicMac, key) in encKeyMap.toMap()) {
                    val plaintext = try {
                        @SuppressLint("GetInstance")
                        val cipher = Cipher.getInstance("AES/ECB/NoPadding")
                        cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(key, "AES"))
                        cipher.doFinal(encrypted)
                    } catch (_: Exception) {
                        continue
                    }

                    if (!isValidCasePlaintext(plaintext)) continue

                    val oldCaseMac = caseAddressByPublicMac[publicMac]?.firstOrNull()
                    if (oldCaseMac != caseMac) {
                        caseAddressByPublicMac[publicMac] = mutableSetOf(caseMac)
                        if (oldCaseMac != null) {
                            caseAddressByPublicMac[publicMac]?.add(oldCaseMac)
                        }
                    }

                    rejectedRandomMac.remove(caseMac)

                    val batteries = buildSet {
                        for ((component, index) in listOf(
                            BatteryComponent.CASE to 3,
                            BatteryComponent.LEFT to 4,
                            BatteryComponent.RIGHT to 5
                        )) {
                            val raw = plaintext[index].toInt() and 0xFF
                            if (raw == 0xFF) continue
                            val charging = raw and 0x80 != 0
                            add(
                                Battery(
                                    component,
                                    raw and 0x7F,
                                    if (charging) BatteryStatus.CHARGING else BatteryStatus.NOT_CHARGING,
                                    BatterySource.CASE_ADVERT,
                                    System.currentTimeMillis()
                                )
                            )
                        }
                    }

                    if (batteries.isNotEmpty()) {
                        val device = (getDevices()[publicMac] as? AppleDevice) ?: continue
                        val previousState = device.state.value

                        val updates = batteries.associateBy { it.component }
                        val merged = (previousState.battery.filterNot { it.component in updates } + updates.values).toSet()

                        device.updateState { state ->
                            state.copy(
                                battery = merged
                            )
                        }

                        if (!previousState.battery.hasSameState(merged)) {
                            Log.d(
                                TAG,
                                "battery changed for ${device.macAddress.toRedactedString()} from case: ${device.state.value.battery.format()}"
                            )
                        }
                    }

                    return true
                }
            }
            offset = payloadEnd
        }
        return false
    }

    private fun isValidCasePlaintext(pt: ByteArray): Boolean {
        if (pt.size != 16) return false
        if (pt[1] != 0x20.toByte() || pt[15] != 0.toByte()) return false
        return (3..5).all { index ->
            val battery = pt[index].toInt() and 0xFF
            battery == 0xFF || (battery and 0x7F) <= 100
        }
    }

    fun getCandidateDevicesForProxConnect(): List<Pair<AppleDevice, AccessoryState>> {
        val now = SystemClock.elapsedRealtime()
        return getDevices().values
            .filterIsInstance<AppleDevice>()
            .mapNotNull { device ->
                val lastSeen = lastSeenMap[device.macAddress] ?: return@mapNotNull null
                val accState = device.state.value.state

                val isFresh = (now - lastSeen) <= timeoutDuration.inWholeMilliseconds
                val isNotConnectedLocally = device.connectionState.value != ConnectionState.CONNECTED &&
                    device.connectionState.value != ConnectionState.CONNECTING
                val hasBudInEar = device.state.value.componentState.any { it.status == ComponentStatus.IN_EAR }

                if (isFresh && isNotConnectedLocally && hasBudInEar) {
                    device to accState
                } else {
                    null
                }
            }
    }
}
