package me.kavishdevar.librepods.database

import androidx.room3.ColumnTypeConverter
import kotlinx.serialization.cbor.Cbor
import kotlinx.serialization.decodeFromByteArray
import kotlinx.serialization.encodeToByteArray
import me.kavishdevar.librepods.core.bluetooth.MacAddress
import me.kavishdevar.librepods.data.app.AccessibilitySettings
import me.kavishdevar.librepods.data.app.FontSettings
import me.kavishdevar.librepods.data.app.IslandSettings
import me.kavishdevar.librepods.core.data.apple.AppleCache
import me.kavishdevar.librepods.core.devices.AppleMetadata
import me.kavishdevar.librepods.core.devices.AppleSettings
import kotlin.time.Instant

object Converters {
    val cbor = Cbor {
        ignoreUnknownKeys = true
    }

    @ColumnTypeConverter
    fun macAddressToString(mac: MacAddress): String = mac.value

    @ColumnTypeConverter
    fun stringToMacAddress(value: String): MacAddress = MacAddress(value)

    @ColumnTypeConverter
    fun appleSettingsToBytes(settings: AppleSettings): ByteArray =
        cbor.encodeToByteArray(settings)

    @ColumnTypeConverter
    fun bytesToAppleSettings(bytes: ByteArray): AppleSettings =
        cbor.decodeFromByteArray(bytes)

    @ColumnTypeConverter
    fun appleMetadataToBytes(metadata: AppleMetadata): ByteArray =
        cbor.encodeToByteArray(metadata)

    @ColumnTypeConverter
    fun bytesToAppleMetadata(bytes: ByteArray): AppleMetadata =
        cbor.decodeFromByteArray(bytes)

    @ColumnTypeConverter
    fun appleCacheToBytes(cache: AppleCache): ByteArray =
        cbor.encodeToByteArray(cache)

    @ColumnTypeConverter
    fun bytesToAppleCache(bytes: ByteArray): AppleCache =
        cbor.decodeFromByteArray(bytes)

    @ColumnTypeConverter
    fun kotlinInstantToLong(instant: Instant): Long =
        instant.toEpochMilliseconds()

    @ColumnTypeConverter
    fun longToKotlinInstant(millis: Long): Instant =
        Instant.fromEpochMilliseconds(millis)

    @ColumnTypeConverter
    fun accessibilitySettingsToBytes(settings: AccessibilitySettings): ByteArray =
        cbor.encodeToByteArray(settings)

    @ColumnTypeConverter
    fun bytesToAccessibilitySettings(bytes: ByteArray): AccessibilitySettings =
        cbor.decodeFromByteArray(bytes)

    @ColumnTypeConverter
    fun fontSettingsToBytes(settings: FontSettings): ByteArray =
        cbor.encodeToByteArray(settings)

    @ColumnTypeConverter
    fun bytesToFontSettings(bytes: ByteArray): FontSettings =
        cbor.decodeFromByteArray(bytes)

    @ColumnTypeConverter
    fun islandSettingsToByte(settings: IslandSettings): ByteArray =
        cbor.encodeToByteArray(settings)

    @ColumnTypeConverter
    fun bytesToIslandSettings(bytes: ByteArray): IslandSettings =
        cbor.decodeFromByteArray(bytes)
}
