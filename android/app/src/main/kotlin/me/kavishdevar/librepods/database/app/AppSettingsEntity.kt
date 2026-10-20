package me.kavishdevar.librepods.database.app

import android.bluetooth.le.ScanSettings
import androidx.annotation.FloatRange
import androidx.compose.ui.graphics.Color
import androidx.room3.ColumnInfo
import androidx.room3.Entity
import androidx.room3.PrimaryKey
import com.materialkolor.PaletteStyle
import me.kavishdevar.librepods.data.app.AccessibilitySettings
import me.kavishdevar.librepods.data.app.ColorStyle
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.data.app.FontSettings
import me.kavishdevar.librepods.data.app.IconStyle
import me.kavishdevar.librepods.data.app.IslandSettings
import me.kavishdevar.librepods.data.app.LibrePodsHazeBlurStyle
import me.kavishdevar.librepods.data.app.NightTheme

@Entity
data class AppSettingsEntity(
    @PrimaryKey
    val id: Int = 0,

    val nightMode: NightTheme = NightTheme.System,
    val designSystem: DesignSystem = DesignSystem.Material,

    @ColumnInfo(defaultValue = "Default")
    val colorStyle: ColorStyle = ColorStyle.Default,

    @ColumnInfo(defaultValue = "TonalSpot")
    val paletteStyle: PaletteStyle = PaletteStyle.TonalSpot,

    val overrideMaterialColor: Color = Color(0xFF007AFF),

    val fontSettings: FontSettings = FontSettings(),

    @ColumnInfo(defaultValue = "Material")
    val iconStyle: IconStyle = IconStyle.Material,

    @ColumnInfo(defaultValue = "1")
    val blurTopBarInMaterial: Boolean = true,

    @ColumnInfo(defaultValue = "0")
    @FloatRange(-1.0, 1.0) val contrastLevel: Double = 0.0,

    @ColumnInfo(defaultValue = "0")
    val amoledMode: Boolean = false,

    @ColumnInfo(defaultValue = "0")
    val progressiveBlurTopBar: Boolean = false,

    @ColumnInfo(defaultValue = "0.5")
    val hazePerformanceMode: Float = 0.5f,

    @ColumnInfo(defaultValue = "Regular")
    val hazeBlurStyle: LibrePodsHazeBlurStyle = LibrePodsHazeBlurStyle.Regular,

    val useHighestRefreshRate: Boolean = false,

    val debugMode: Boolean = false,

    val bleScanMode: Int = ScanSettings.SCAN_MODE_BALANCED,
    val bleReportDelay: Long = 0,

    val swipeAnywhereForBack: Boolean = true,

    val accessibilitySettings: AccessibilitySettings = AccessibilitySettings(),
    val islandSettings: IslandSettings = IslandSettings()
)
