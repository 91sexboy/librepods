package me.kavishdevar.librepods.data.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance


sealed interface OverlayBackgroundColor {
    data object Surface: OverlayBackgroundColor
    data object SurfaceContainer: OverlayBackgroundColor
    data class Custom(val colorValue: Long): OverlayBackgroundColor

    data class Default(val overlayType: OverlayType): OverlayBackgroundColor
}

@Composable
fun OverlayBackgroundColor.resolve(): Color {
    return when (this) {
        OverlayBackgroundColor.Surface -> MaterialTheme.colorScheme.surface
        OverlayBackgroundColor.SurfaceContainer -> MaterialTheme.colorScheme.surfaceContainer
        is OverlayBackgroundColor.Custom -> Color(colorValue.toULong())
        is OverlayBackgroundColor.Default -> {
            when (this.overlayType) {
                OverlayType.Island -> Color.Black
                OverlayType.ProximityCard -> if (MaterialTheme.colorScheme.surface.luminance() > 0.5f) Color.White else Color(0xFF1C1B1E) // apple's transparency channel in prox loop...
            }
        }
    }
}

enum class OverlayBatteryStyle {
    NONE,
    TEXT,
    CIRCLE,
    CIRCLE_TEXT,
}

enum class OverlayType {
    Island,
    ProximityCard
}
