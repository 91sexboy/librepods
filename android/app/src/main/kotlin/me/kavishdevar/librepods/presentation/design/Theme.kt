/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

package me.kavishdevar.librepods.presentation.design

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import com.materialkolor.material3.rememberDynamicColorScheme
import dev.chrisbanes.haze.HazePerformanceMode
import dev.chrisbanes.haze.HazeProgressive
import dev.chrisbanes.haze.LocalHazePerformanceMode
import dev.chrisbanes.haze.blur.HazeBlurDefaults
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import dev.chrisbanes.haze.blur.materials.HazeMaterials
import dev.chrisbanes.haze.glass.GlassAccessibilitySettings
import dev.chrisbanes.haze.glass.LocalGlassAccessibilitySettings
import me.kavishdevar.librepods.data.app.AccessibilitySettings
import me.kavishdevar.librepods.data.app.ColorStyle
import me.kavishdevar.librepods.data.app.Design
import me.kavishdevar.librepods.data.app.FontFamilyOption
import me.kavishdevar.librepods.data.app.FontSettings
import me.kavishdevar.librepods.data.app.IconStyle
import me.kavishdevar.librepods.data.app.LibrePodsHazeBlurStyle
import me.kavishdevar.librepods.presentation.design.typography.InterTypography
import me.kavishdevar.librepods.presentation.design.typography.RobotoFlexExpressiveTypography
import me.kavishdevar.librepods.presentation.design.typography.RobotoFlexTypography
import me.kavishdevar.librepods.presentation.icons.AppleIcons
import me.kavishdevar.librepods.presentation.icons.LocalIcons
import me.kavishdevar.librepods.presentation.icons.MaterialIcons

val ColorScheme.sectionHeader: Color
    get() = onBackground.copy(alpha = 0.6f)

private val DefaultDarkColorScheme = darkColorScheme(
    background = Color(0xFF000000),
    onBackground = Color(0xFFFFFFFF),
    surfaceContainer = Color(0xFF000000),
    onSurface = Color(0xFFFFFFFF),
    surfaceDim = Color(0x40888888),
    primary = Color(0xFF0091FF),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFF003258),
    onPrimaryContainer = Color(0xFFB3D9FF),
    secondaryContainer = Color(0xFF366AA8),
    onSecondaryContainer = Color(0xFFF2F0E8),
    tertiary = Color(0xFFEA7B00),
    scrim = Color(0x8C000000),
    surfaceContainerLow = Color(0xFF2C2C2E),
    surfaceContainerHigh = Color(0xFF1C1C1E),
    surfaceContainerHighest = Color(0xFF545454)
)

private val DefaultLightColorScheme = lightColorScheme(
    background = Color(0xFFF2F2F7),
    onBackground = Color(0xFF000000),
    surfaceContainer = Color(0xFFF2F2F7),
    onSurface = Color(0xFF000000),
    surfaceDim = Color(0x40D9D9D9),
    primary = Color(0xFF0088FF),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFB3D9FF),
    onPrimaryContainer = Color(0xFF003258),
    secondaryContainer = Color(0xFF6BC0FF),
    onSecondaryContainer = Color(0xFF183B56),
    tertiary = Color(0xFFEA7B00),
    scrim = Color(0xD9F2F2F7),
    surfaceContainerLow = Color(0xFFE7E7E7),
    surfaceContainerHigh = Color(0xFFFFFFFF),
    surfaceContainerHighest = Color(0xFF8D8D8D)
)

@Composable
fun LibrePodsTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    design: Design = Design(),
    accessibilitySettings: AccessibilitySettings = AccessibilitySettings(),
    fontSettings: FontSettings = FontSettings(),
    content: @Composable () -> Unit
) {
    val typography = when(fontSettings.fontFamilyOption) {
        FontFamilyOption.SystemDefault -> Typography()
        FontFamilyOption.RobotoFlex -> RobotoFlexTypography
        FontFamilyOption.RobotoFlexExpressive -> RobotoFlexExpressiveTypography
        FontFamilyOption.Inter -> InterTypography
    }

    CompositionLocalProvider(
        LocalDesign provides design,
        LocalAccessibilitySettings provides accessibilitySettings,
        LocalIcons provides when (design.iconStyle) {
            IconStyle.Material -> MaterialIcons
            IconStyle.Apple -> AppleIcons
        },
        LocalGlassAccessibilitySettings provides GlassAccessibilitySettings(
            reduceTransparency = accessibilitySettings.reduceTransparency,
            increaseContrast = accessibilitySettings.increaseContrast,
            showBorders = accessibilitySettings.showBorders,
        ),
        LocalHazePerformanceMode provides HazePerformanceMode.Fixed(design.hazePerformanceMode),
    ) {
        val context = LocalContext.current
        val colorScheme = when (design.colorStyle) {
            ColorStyle.Custom -> {
                rememberDynamicColorScheme(
                    seedColor = design.overrideMaterialColor,
                    isDark = darkTheme,
                    contrastLevel = design.contrastLevel,
                    isAmoled = design.amoledMode,
                    style = design.paletteStyle,
                )
            }
            ColorStyle.Default -> if (darkTheme) DefaultDarkColorScheme else DefaultLightColorScheme
            ColorStyle.Wallpaper -> if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }

        MaterialExpressiveTheme(
            colorScheme = colorScheme,
            motionScheme = MotionScheme.expressive(),
            typography = typography
        ) {
            val containerColor = if (design.colorStyle == ColorStyle.Custom) MaterialTheme.colorScheme.surface.copy(0.3f) else MaterialTheme.colorScheme.surfaceContainer.copy(0.3f)

            val hazeBlurStyle = when(design.hazeBlurStyle) {
                LibrePodsHazeBlurStyle.UltraThin -> HazeMaterials.ultraThin(containerColor).then {
                    progressive(if (design.progressiveBlurEnabled) HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f) else null)
                }
                LibrePodsHazeBlurStyle.Thin -> HazeMaterials.thin(containerColor).then {
                    progressive(if (design.progressiveBlurEnabled) HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f) else null)
                }
                LibrePodsHazeBlurStyle.Regular -> HazeMaterials.regular(containerColor).then {
                    progressive(if (design.progressiveBlurEnabled) HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f) else null)
                }
                LibrePodsHazeBlurStyle.Thick -> HazeMaterials.thick(containerColor).then {
                    progressive(if (design.progressiveBlurEnabled) HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f) else null)
                }
                LibrePodsHazeBlurStyle.UltraThick -> HazeMaterials.ultraThick(containerColor).then {
                    progressive(if (design.progressiveBlurEnabled) HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f) else null)
                }
                LibrePodsHazeBlurStyle.Custom -> HazeBlurDefaults.style.then {
                    backgroundColor(containerColor)
                    blurRadius(design.hazeBlurRadius)
                    noiseFactor(design.hazeNoiseFactor)
                    alpha(design.alpha)
                    progressive(if (design.progressiveBlurEnabled) HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f) else null)
                }
            }

            CompositionLocalProvider(
                LocalHazeBlurStyle provides hazeBlurStyle,
            ) {
                content()
            }
        }
    }
}
