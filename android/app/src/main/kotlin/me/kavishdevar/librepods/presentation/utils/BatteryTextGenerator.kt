package me.kavishdevar.librepods.presentation.utils

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.core.devices.AirPodsSpec
import me.kavishdevar.librepods.core.devices.Battery
import me.kavishdevar.librepods.core.devices.BatteryComponent
import me.kavishdevar.librepods.core.devices.BatteryStatus
import me.kavishdevar.librepods.core.devices.DeviceComponent
import me.kavishdevar.librepods.presentation.icons.RichText
import me.kavishdevar.librepods.presentation.icons.richText
import kotlin.math.absoluteValue
import kotlin.time.Duration.Companion.seconds

@Composable
fun rememberRelativeTime(timestampMillis: Long): String {
    var currentTime by remember { mutableLongStateOf(System.currentTimeMillis()) }

    LaunchedEffect(timestampMillis) {
        while (isActive) {
            delay(1.seconds)
            currentTime = System.currentTimeMillis()
        }
    }

    val diffMillis = currentTime - timestampMillis
    if (diffMillis < 1000L) return "now"

    val totalSeconds = diffMillis / 1000
    val hours = totalSeconds / 3600
    val minutes = (totalSeconds % 3600) / 60
    val seconds = totalSeconds % 60

    return when {
        hours > 0 -> "${hours}h ${minutes}m ago"
        minutes > 0 -> "${minutes}m ${seconds}s ago"
        else -> "${seconds}s ago"
    }
}

@Composable
fun createAirPodsBatteryRichText(
    battery: Set<Battery>,
    airPodsSpec: AirPodsSpec
): RichText {
    val airPodsIconName = airPodsSpec.genericIconName
    val leftIconName = airPodsSpec.components.find { it.type == DeviceComponent.LEFT }?.iconName ?: "AirPodsPro3Left"
    val rightIconName = airPodsSpec.components.find { it.type == DeviceComponent.RIGHT }?.iconName ?: "AirPodsPro3Right"
    val airPodsCaseIconName = (airPodsSpec.components.find { it.type == DeviceComponent.CASE }?.iconName?: "AirPodsPro3Case") + "Fill"
    val headsetIconName = airPodsSpec.components.find { it.type == DeviceComponent.HEADSET }?.iconName ?: "AirPodsMax"

    val left = battery.find { it.component == BatteryComponent.LEFT }
    val leftLevel = left?.level ?: 0

    val right = battery.find { it.component == BatteryComponent.RIGHT }
    val rightLevel = right?.level ?: 0

    val case = battery.find { it.component == BatteryComponent.CASE }
    val caseLevel = case?.level ?: 0

    val headset = battery.find { it.component == BatteryComponent.HEADSET }
    val headsetLevel = headset?.level ?: 0

    val individualIcons = ((leftLevel - rightLevel).absoluteValue >= 5 || left?.status != right?.status) || LocalDebugMode.current

    val budsBatteryText = if (individualIcons) {
        val leftBatteryText =
            if (left != null && leftLevel > 0) {
                val statusIcon = when (left.status) {
                    BatteryStatus.CHARGING, BatteryStatus.OPTIMIZED_CHARGING -> "\\icon{BoltCircle}"
                    BatteryStatus.NOT_CHARGING -> "\\icon{Circle}"
                    BatteryStatus.UNKNOWN, BatteryStatus.DISCONNECTED -> "\\icon{CircleDotted}"
                }
                "\\icon{$leftIconName} " + statusIcon + " ${leftLevel}%" + if (LocalDebugMode.current) " (${left.source}, ${rememberRelativeTime(left.timestamp)})\n" else ""
            } else ""

        val rightBatteryText =
            if (right != null && rightLevel > 0) {
                val statusIcon = when (right.status) {
                    BatteryStatus.CHARGING, BatteryStatus.OPTIMIZED_CHARGING -> "\\icon{BoltCircle}"
                    BatteryStatus.NOT_CHARGING -> "\\icon{Circle}"
                    BatteryStatus.UNKNOWN, BatteryStatus.DISCONNECTED -> "\\icon{CircleDotted}"
                }
                "\\icon{$rightIconName} " + statusIcon + " ${rightLevel}%" + if (LocalDebugMode.current) " (${right.source}, ${rememberRelativeTime(right.timestamp)})\n" else ""

            } else ""

        leftBatteryText + (if (LocalDebugMode.current) "" else "  ") + rightBatteryText
    } else {
        val statusIcon = when {
            left?.status == BatteryStatus.CHARGING || left?.status == BatteryStatus.OPTIMIZED_CHARGING -> "\\icon{BoltCircle}"
            left != null && (left.status == BatteryStatus.DISCONNECTED) -> "\\icon{CircleDotted}"
            else -> "\\icon{Circle}"
        }
        "\\icon{${airPodsIconName}} " + statusIcon + " ${
            leftLevel.coerceAtMost(
                rightLevel
            )
        }%"
    }

    val caseBatteryText = if (case != null && caseLevel > 0) {
        val statusIcon = when (case.status) {
            BatteryStatus.CHARGING, BatteryStatus.OPTIMIZED_CHARGING -> "\\icon{BoltCircle}"
            BatteryStatus.NOT_CHARGING -> "\\icon{Circle}"
            BatteryStatus.UNKNOWN, BatteryStatus.DISCONNECTED -> "\\icon{CircleDotted}"
        }
        "\\icon{$airPodsCaseIconName} " + statusIcon + " ${caseLevel}%" + if (LocalDebugMode.current) " (${case.source}, ${rememberRelativeTime(case.timestamp)})" else ""
    } else ""

    return richText(
        if (left != null && right != null && case != null) {
            budsBatteryText + (if (LocalDebugMode.current) "" else "  ") + caseBatteryText
        } else if (case != null) {
            caseBatteryText
        } else {
            if (headset != null) {
                val statusIcon = when (headset.status) {
                    BatteryStatus.CHARGING, BatteryStatus.OPTIMIZED_CHARGING -> "\\icon{BoltCircle}"
                    BatteryStatus.NOT_CHARGING -> "\\icon{Circle}"
                    BatteryStatus.UNKNOWN, BatteryStatus.DISCONNECTED -> "\\icon{CircleDotted}"
                }
                "\\icon{$headsetIconName} " + statusIcon + " ${headsetLevel}%"
            } else {
                stringResource(R.string.battery_not_available)
            }
        }
    )
}
