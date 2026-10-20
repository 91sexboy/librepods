package me.kavishdevar.librepods.presentation.screens.settings.appearance

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.data.app.LibrePodsHazeBlurStyle
import me.kavishdevar.librepods.presentation.components.primitives.StyledList
import me.kavishdevar.librepods.presentation.components.primitives.StyledScaffold
import me.kavishdevar.librepods.presentation.components.primitives.StyledToggle
import me.kavishdevar.librepods.presentation.components.primitives.styledListItem
import me.kavishdevar.librepods.presentation.design.LocalDesign
import me.kavishdevar.librepods.presentation.viewmodel.AppSettingsViewModel

@Composable
fun HazeSettingsRoute(
    viewModel: AppSettingsViewModel,
    navigateBack: (() -> Unit)?,
) {
    val uiState by viewModel.uiState.collectAsState()
    val settings = uiState.settings

    HazeSettingsScreen(
        navigateBack = navigateBack,
        blurTopBarInMaterial = settings.blurTopBarInMaterial,
        onBlurTopBarInMaterialChanged = { blurTopBarInMaterial ->
            viewModel.updateSettings {
                it.copy(blurTopBarInMaterial = blurTopBarInMaterial)
            }
        },
        progressiveBlurTopBar = settings.progressiveBlurTopBar,
        onProgressiveBlurTopBarChanged = { progressiveBlurTopBar ->
            viewModel.updateSettings {
                it.copy(progressiveBlurTopBar = progressiveBlurTopBar)
            }
        },
        hazeBlurStyle = settings.hazeBlurStyle,
        onHazeBlurStyleChanged = { hazeBlurStyle ->
            viewModel.updateSettings {
                it.copy(hazeBlurStyle = hazeBlurStyle)
            }
        }
    )
}

@Composable
fun HazeSettingsScreen(
    navigateBack: (() -> Unit)? = null,
    blurTopBarInMaterial: Boolean,
    onBlurTopBarInMaterialChanged: (Boolean) -> Unit,
    progressiveBlurTopBar: Boolean,
    onProgressiveBlurTopBarChanged: (Boolean) -> Unit,
    hazeBlurStyle: LibrePodsHazeBlurStyle,
    onHazeBlurStyleChanged: (LibrePodsHazeBlurStyle) -> Unit,
) {
    val scrollState = rememberScrollState()

    StyledScaffold(
        title = stringResource(R.string.blur_and_glass_effects),
        navigateBack = navigateBack
    ) { topPadding, bottomPadding ->
        Column(
            modifier = Modifier
                .padding(horizontal = 16.dp)
                .verticalScroll(scrollState),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Spacer(modifier = Modifier.height(topPadding))

            AnimatedVisibility(
                visible = LocalDesign.current.system == DesignSystem.Material
            ) {
                StyledToggle(
                    label = stringResource(R.string.blur_content_behind_top_bar),
                    checked = blurTopBarInMaterial,
                    onCheckedChange = onBlurTopBarInMaterialChanged
                )
            }

            StyledToggle(
                label = stringResource(R.string.use_progressive_blur_for_top_bar),
                checked = progressiveBlurTopBar,
                onCheckedChange = onProgressiveBlurTopBarChanged
            )

            StyledList(title = stringResource(R.string.blur_style)) {
                styledListItem(
                    contentText = stringResource(R.string.blur_style_ultra_thin),
                    selected = hazeBlurStyle == LibrePodsHazeBlurStyle.UltraThin,
                    onClick = { onHazeBlurStyleChanged(LibrePodsHazeBlurStyle.UltraThin) },
                )
                styledListItem(
                    contentText = stringResource(R.string.blur_style_thin),
                    selected = hazeBlurStyle == LibrePodsHazeBlurStyle.Thin,
                    onClick = { onHazeBlurStyleChanged(LibrePodsHazeBlurStyle.Thin) },
                )
                styledListItem(
                    contentText = stringResource(R.string.blur_style_regular),
                    selected = hazeBlurStyle == LibrePodsHazeBlurStyle.Regular,
                    onClick = { onHazeBlurStyleChanged(LibrePodsHazeBlurStyle.Regular) },
                )
                styledListItem(
                    contentText = stringResource(R.string.blur_style_thick),
                    selected = hazeBlurStyle == LibrePodsHazeBlurStyle.Thick,
                    onClick = { onHazeBlurStyleChanged(LibrePodsHazeBlurStyle.Thick) },
                )
                styledListItem(
                    contentText = stringResource(R.string.blur_style_ultra_thick),
                    selected = hazeBlurStyle == LibrePodsHazeBlurStyle.UltraThick,
                    onClick = { onHazeBlurStyleChanged(LibrePodsHazeBlurStyle.UltraThick) },
                )
//                styledListItem(
//                    contentText = "Custom",
//                    selected = hazeBlurStyle == LibrePodsHazeBlurStyle.Custom,
//                    onClick = { onHazeBlurStyleChanged(LibrePodsHazeBlurStyle.Custom) },
//                )
            }

            Spacer(modifier = Modifier.height(bottomPadding))
        }
    }
}
