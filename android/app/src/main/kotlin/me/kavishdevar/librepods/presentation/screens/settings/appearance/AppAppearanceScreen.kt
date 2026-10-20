package me.kavishdevar.librepods.presentation.screens.settings.appearance

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
import me.kavishdevar.librepods.data.app.FontFamilyOption
import me.kavishdevar.librepods.data.app.FontSettings
import me.kavishdevar.librepods.data.app.IconStyle
import me.kavishdevar.librepods.data.app.NightTheme
import me.kavishdevar.librepods.presentation.components.primitives.StyledList
import me.kavishdevar.librepods.presentation.components.primitives.StyledListItem
import me.kavishdevar.librepods.presentation.components.primitives.StyledScaffold
import me.kavishdevar.librepods.presentation.components.primitives.styledListItem
import me.kavishdevar.librepods.presentation.viewmodel.AppSettingsViewModel

@Composable
fun AppAppearanceSettingsRoute(
    viewModel: AppSettingsViewModel,
    navigateBack: (() -> Unit)?,
    navigateToAppColorSettingsScreen: () -> Unit,
    navigateToHazeSettingsScreen: () -> Unit
) {
    val uiState by viewModel.uiState.collectAsState()
    val settings = uiState.settings

    AppAppearanceSettingsScreen(
        navigateBack = navigateBack,
        navigateToAppColorSettingsScreen = navigateToAppColorSettingsScreen,
        navigateToHazeSettingsScreen = navigateToHazeSettingsScreen,
        designSystem = settings.designSystem,
        onDesignSystemChanged = { designSystem ->
            viewModel.updateSettings {
                it.copy(designSystem = designSystem)
            }
        },
        nightMode = settings.nightMode,
        onNightModeChanged = { nightMode ->
            viewModel.updateSettings {
                it.copy(nightMode = nightMode)
            }
        },
        iconStyle = settings.iconStyle,
        onIconStyleChanged = { iconStyle ->
            viewModel.updateSettings {
                it.copy(iconStyle = iconStyle)
            }
        },
        fontSettings = settings.fontSettings,
        onFontSettingsChanged = { fontSettings ->
            viewModel.updateSettings {
                it.copy(fontSettings = fontSettings)
            }
        }
    )
}

@Composable
fun AppAppearanceSettingsScreen(
    navigateBack: (() -> Unit)? = null,
    navigateToAppColorSettingsScreen: () -> Unit,
    navigateToHazeSettingsScreen: () -> Unit,
    designSystem: DesignSystem,
    onDesignSystemChanged: (DesignSystem) -> Unit,
    nightMode: NightTheme,
    onNightModeChanged: (NightTheme) -> Unit,
    iconStyle: IconStyle,
    onIconStyleChanged: (IconStyle) -> Unit,
    fontSettings: FontSettings,
    onFontSettingsChanged: (FontSettings) -> Unit,
) {
    val scrollState = rememberScrollState()

    StyledScaffold(
        title = stringResource(R.string.appearance),
        navigateBack = navigateBack
    ) { topPadding, bottomPadding ->
        Column(
            modifier = Modifier
                .padding(horizontal = 16.dp)
                .verticalScroll(scrollState),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Spacer(modifier = Modifier.height(topPadding))

            StyledList {
                styledListItem(
                    contentText = stringResource(R.string.light),
                    selected = nightMode == NightTheme.Light,
                    onClick = { onNightModeChanged(NightTheme.Light) },
                )

                styledListItem(
                    contentText = stringResource(R.string.system),
                    selected = nightMode == NightTheme.System,
                    onClick = { onNightModeChanged(NightTheme.System) },
                )

                styledListItem(
                    contentText = stringResource(R.string.dark),
                    selected = nightMode == NightTheme.Dark,
                    onClick = { onNightModeChanged(NightTheme.Dark) },
                )
            }

            StyledList(
                title = stringResource(R.string.design_system),
            ) {
                styledListItem(
                    contentText = stringResource(R.string.apple),
                    selected = designSystem == DesignSystem.Apple,
                    onClick = {
                        onDesignSystemChanged(DesignSystem.Apple)
                        onFontSettingsChanged(fontSettings.copy(fontFamilyOption = FontFamilyOption.Inter))
                        onIconStyleChanged(IconStyle.Apple)
                    },
                )

                styledListItem(
                    contentText = stringResource(R.string.material3e),
                    selected = designSystem == DesignSystem.Material,
                    onClick = {
                        onDesignSystemChanged(DesignSystem.Material)
                        onFontSettingsChanged(fontSettings.copy(fontFamilyOption = FontFamilyOption.RobotoFlexExpressive))
                        onIconStyleChanged(IconStyle.Material)
                    },
                )
            }

            StyledList(title = stringResource(R.string.icons)) {
                styledListItem(
                    contentText = stringResource(R.string.material_symbols),
                    selected = iconStyle == IconStyle.Material,
                    onClick = { onIconStyleChanged(IconStyle.Material) },
                )
                styledListItem(
                    contentText = stringResource(R.string.apple),
                    selected = iconStyle == IconStyle.Apple,
                    onClick = { onIconStyleChanged(IconStyle.Apple) },
                )
            }

            StyledList(title = stringResource(R.string.font)) {
                styledListItem(
                    contentText = stringResource(R.string.system),
                    selected = fontSettings.fontFamilyOption == FontFamilyOption.SystemDefault,
                    onClick = { onFontSettingsChanged(fontSettings.copy(fontFamilyOption = FontFamilyOption.SystemDefault)) },
                )
                styledListItem(
                    contentText = stringResource(R.string.roboto_flex),
                    selected = fontSettings.fontFamilyOption == FontFamilyOption.RobotoFlex,
                    onClick = { onFontSettingsChanged(fontSettings.copy(fontFamilyOption = FontFamilyOption.RobotoFlex)) },
                )
                styledListItem(
                    contentText = stringResource(R.string.roboto_flex_expressive),
                    selected = fontSettings.fontFamilyOption == FontFamilyOption.RobotoFlexExpressive,
                    onClick = { onFontSettingsChanged(fontSettings.copy(fontFamilyOption = FontFamilyOption.RobotoFlexExpressive)) },
                )
                styledListItem(
                    contentText = stringResource(R.string.inter),
                    selected = fontSettings.fontFamilyOption == FontFamilyOption.Inter,
                    onClick = { onFontSettingsChanged(fontSettings.copy(fontFamilyOption = FontFamilyOption.Inter)) },
                )
            }

            StyledListItem(
                contentText = stringResource(R.string.color),
                onClick = navigateToAppColorSettingsScreen
            )

            StyledListItem(
                contentText = stringResource(R.string.blur_and_glass_effects),
                onClick = navigateToHazeSettingsScreen
            )

            Spacer(modifier = Modifier.height(bottomPadding))
        }
    }
}
