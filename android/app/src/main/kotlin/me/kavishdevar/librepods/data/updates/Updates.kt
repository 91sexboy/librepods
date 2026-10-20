package me.kavishdevar.librepods.data.updates

import androidx.compose.runtime.Composable
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.presentation.design.LocalDesign
import me.kavishdevar.librepods.presentation.screens.apple.AppleSettingsScreenPreviewMaterial
import me.kavishdevar.librepods.presentation.screens.apple.EqualizerScreenPreviewApple
import me.kavishdevar.librepods.presentation.screens.apple.EqualizerScreenPreviewMaterial

val update1_0_0 = listOf(
        UpdateItem(
            titleRes = R.string.material3e,
            descriptionRes = R.string.update_m3e_description,
            demoComposeable = @Composable {
                AppleSettingsScreenPreviewMaterial()
            }
        ),
        UpdateItem(
            titleRes = R.string.equalizer,
            descriptionRes = R.string.update_equalizer_description,
            demoComposeable = @Composable {
                val design = LocalDesign.current
                
                when (design.system) {
                    DesignSystem.Apple -> {
                        EqualizerScreenPreviewApple()
                    }
                    DesignSystem.Material -> {
                        EqualizerScreenPreviewMaterial()
                    }
                }
            }
        ),
    )

val updates = update1_0_0
