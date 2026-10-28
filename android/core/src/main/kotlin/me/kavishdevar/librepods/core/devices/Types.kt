package me.kavishdevar.librepods.core.devices

import android.os.Parcelable
import kotlinx.parcelize.Parcelize

// TODO: differentiate between bluetooth connected and oem-specific stuff connected. available is reserved for BLE based things, hence not used bluetooth connected
enum class ConnectionState {
    DISCONNECTED,
    DISCONNECTING,
    AVAILABLE,
    CONNECTING,
    CONNECTED,
}

enum class BatteryComponent {
    LEFT,
    RIGHT,
    CASE,
    HEADSET;

    companion object {
        fun fromAirPodsByte(value: Byte): BatteryComponent {
            return when (value.toInt()) {
                1 -> HEADSET
                4 -> LEFT
                2 -> RIGHT
                8 -> CASE
                else -> throw IllegalArgumentException("Unknown battery component value: $value")
            }
        }
    }
}

enum class BatteryStatus {
    UNKNOWN, // got this from pro 3 when one of the airpods had crashed and the other was still connected
    CHARGING,
    NOT_CHARGING,
    DISCONNECTED,
    OPTIMIZED_CHARGING;

    companion object {
        fun fromAirPodsByte(value: Byte): BatteryStatus {
            return when (value.toInt()) {
                0 -> UNKNOWN
                1 -> CHARGING
                2 -> NOT_CHARGING
                4 -> DISCONNECTED
                5 -> OPTIMIZED_CHARGING
                else -> throw IllegalArgumentException("Unknown battery status value: $value")
            }
        }
    }
}

enum class BatterySource {
    AACP,
    CASE_ADVERT,
    BUDS_ADVERT
}

@Parcelize
data class Battery(
    val component: BatteryComponent,
    val level: Int,
    val status: BatteryStatus,
    val source: BatterySource,
    val timestamp: Long,
): Parcelable {
    override fun toString(): String {
        return "$component: $status at $level% from $source ($timestamp)"
    }
}

fun Set<Battery>.hasSameState(other: Set<Battery>): Boolean {
    val currentByComponent = associateBy { it.component }
    val otherByComponent = other.associateBy { it.component }

    return currentByComponent.keys == otherByComponent.keys && currentByComponent.all { (component, current) ->
        val otherBattery = otherByComponent[component] ?: return@all false
        current.level == otherBattery.level && current.status == otherBattery.status
    }
}

fun Set<Battery>.format(): String {
    return this.joinToString(separator = "\n") { it.toString() }
}

// TODO: use this for battery too. BatteryComponent is very redundant
enum class DeviceComponent {
    LEFT,
    RIGHT,
    CASE,
    HEADSET
}

enum class ComponentStatus {
    IN_EAR,
    OUT_OF_EAR,
    IN_CASE,
    DISCONNECTED;

    companion object {
        fun fromAirPodsByte(value: Byte): ComponentStatus {
            return when (value.toInt()) {
                0 -> IN_EAR
                1 -> OUT_OF_EAR
                2 -> IN_CASE
                3 -> DISCONNECTED
                else -> throw IllegalArgumentException("Unknown device status value: $value")
            }
        }
    }

    fun toAirPodsByte(): Byte {
        return when (this) {
            IN_EAR -> 0
            OUT_OF_EAR -> 1
            IN_CASE -> 2
            DISCONNECTED -> 3
        }.toByte()
    }
}

data class DeviceComponentState(
    val component: DeviceComponent,
    val status: ComponentStatus
)

enum class PacketDestination {
    DEVICE,
    HOST;
}

enum class NoiseControlMode {
    OFF,  NOISE_CANCELLATION, TRANSPARENCY, ADAPTIVE
}
