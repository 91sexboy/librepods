package me.kavishdevar.librepods.data.app

import androidx.annotation.FloatRange
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.materialkolor.PaletteStyle

data class Design(
    val system: DesignSystem = DesignSystem.Material,
    val iconStyle: IconStyle = IconStyle.Material,

    val blurTopBarInMaterial: Boolean = false,
    val progressiveBlurEnabled: Boolean = false,

    val hazePerformanceMode: Float = 0.5f,
    val hazeBlurStyle: LibrePodsHazeBlurStyle = LibrePodsHazeBlurStyle.Thin,

    val hazeBlurRadius: Dp = 20.dp,
    val hazeNoiseFactor: Float = 0.15f,
    val alpha: Float = 1f,

    val colorStyle: ColorStyle = ColorStyle.Default,
    val paletteStyle: PaletteStyle = PaletteStyle.TonalSpot,
    val overrideMaterialColor: Color = Color(0xFF007AFF),
    val amoledMode: Boolean = true,
    @FloatRange(-1.0, 1.0) val contrastLevel: Double = 0.0
)
