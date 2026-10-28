package me.kavishdevar.librepods.data.app

import kotlinx.serialization.Serializable

@Serializable
data class ProximityCardSettings(
    val width: Float = 1000f,
    val height: Float = 800f,

    val borderRadius: Float = 78f,
    val borderWidth: Float = 0f,

    val backgroundColor: OverlayBackgroundColor = OverlayBackgroundColor.Default(OverlayType.ProximityCard),
    val backgroundAlpha: Float = 0.95f,

    val showConnectButton: Boolean = true,

    val batteryStyle: OverlayBatteryStyle = OverlayBatteryStyle.CIRCLE_TEXT,

    val offsetY: Float = 0f,

    val centerOnTablet: Boolean = true,

    val useVideo: Boolean = false,

    val requireBothBudsInCase: Boolean = true,

    val showCloseButton: Boolean = true
)
