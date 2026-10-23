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

package me.kavishdevar.librepods.presentation.screens.apple

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
import me.kavishdevar.librepods.presentation.components.primitives.StyledListItem
import me.kavishdevar.librepods.presentation.components.primitives.StyledListItemOrientation
import me.kavishdevar.librepods.presentation.components.primitives.StyledScaffold
import me.kavishdevar.librepods.presentation.components.primitives.StyledToggle
import me.kavishdevar.librepods.presentation.viewmodel.AppleViewModel

@Composable
fun OverlaySettingsRoute(
    viewModel: AppleViewModel,
    navigateBack: (() -> Unit)?,
    navigateToAppSettings: () -> Unit
) {
    val uiState by viewModel.uiState.collectAsState()

    OverlaySettingsScreen(
        navigateBack = navigateBack,
        navigateToAppSettings = navigateToAppSettings,
        islandEnabled = uiState.settings.showIslandPopup,
        onIslandEnabledChanged = {
            viewModel.updateSettings {
                it.copy(
                    showIslandPopup = it.showIslandPopup
                )
            }
        },
        proxCardEnabled = uiState.settings.showBottomSheetPopup,
        onProxCardEnabledChanged = {
            viewModel.updateSettings {
                it.copy(
                    showBottomSheetPopup = it.showBottomSheetPopup
                )
            }
        }
    )
}
@Composable
fun OverlaySettingsScreen(
    navigateBack: (() -> Unit)?,
    navigateToAppSettings: () -> Unit,
    islandEnabled: Boolean,
    onIslandEnabledChanged: (Boolean) -> Unit,
    proxCardEnabled: Boolean,
    onProxCardEnabledChanged: (Boolean) -> Unit,
) {
    val scrollState = rememberScrollState()

    StyledScaffold(
        title = stringResource(R.string.overlay_settings),
        navigateBack = navigateBack
    ) { topPadding, bottomPadding ->
        Column(
            modifier = Modifier
                .padding(horizontal = 16.dp)
                .verticalScroll(scrollState),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Spacer(modifier = Modifier.height(topPadding))

            StyledToggle(
                label = stringResource(R.string.island_popup),
                description = stringResource(R.string.island_popup_description),
                checked = islandEnabled,
                onCheckedChange = onIslandEnabledChanged
            )

            StyledToggle(
                label = stringResource(R.string.proximity_card),
                description = stringResource(R.string.proximity_card_description),
                checked = proxCardEnabled,
                onCheckedChange = onProxCardEnabledChanged
            )

            StyledListItem(
                contentText = stringResource(R.string.more_settings),
                supportingText = stringResource(R.string.overlay_more_settings_description),
                onClick = navigateToAppSettings,
                orientation = StyledListItemOrientation.Vertical
            )

            Spacer(modifier = Modifier.height(bottomPadding))
        }
    }
}
