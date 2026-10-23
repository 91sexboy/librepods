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
import me.kavishdevar.librepods.presentation.components.primitives.StyledList
import me.kavishdevar.librepods.presentation.components.primitives.StyledScaffold
import me.kavishdevar.librepods.presentation.components.primitives.StyledToggle
import me.kavishdevar.librepods.presentation.components.primitives.styledListItem
import me.kavishdevar.librepods.presentation.viewmodel.AppleViewModel

@Composable
fun ProxConnectSettingsRoute(
    viewModel: AppleViewModel,
    navigateBack: (() -> Unit)?,
) {
    val uiState by viewModel.uiState.collectAsState()

    ProxConnectSettingsScreen(
        navigateBack = navigateBack,

        alwaysConnectWhenAccessoryDisconnected = uiState.settings.alwaysConnectWhenAccessoryDisconnected,
        onAlwaysConnectWhenAccessoryDisconnectedChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    alwaysConnectWhenAccessoryDisconnected = enabled
                )
            }
        },
        connectOnPhoneWhenAccessoryDisconnected = uiState.settings.connectOnPhoneWhenAccessoryDisconnected,
        onConnectOnPhoneWhenAccessoryDisconnectedChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    connectOnPhoneWhenAccessoryDisconnected = enabled
                )
            }
        },
        connectOnPhoneWhenAccessoryIdle  = uiState.settings.connectOnPhoneWhenAccessoryIdle,
        onConnectOnPhoneWhenAccessoryIdleChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    connectOnPhoneWhenAccessoryIdle = enabled
                )
            }
        },
        connectOnPhoneWhenAccessoryMusic  = uiState.settings.connectOnPhoneWhenAccessoryMusic,
        onConnectOnPhoneWhenAccessoryMusicChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    connectOnPhoneWhenAccessoryMusic = enabled
                )
            }
        },

        connectOnMediaWhenAccessoryDisconnected = uiState.settings.connectOnMediaWhenAccessoryDisconnected,
        onConnectOnMediaWhenAccessoryDisconnectedChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    connectOnMediaWhenAccessoryDisconnected = enabled
                )
            }
        },
        connectOnMediaWhenAccessoryIdle  = uiState.settings.connectOnMediaWhenAccessoryIdle,
        onConnectOnMediaWhenAccessoryIdleChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    connectOnMediaWhenAccessoryIdle = enabled
                )
            }
        },
        connectOnMediaWhenAccessoryMusic  = uiState.settings.connectOnMediaWhenAccessoryMusic,
        onConnectOnMediaWhenAccessoryMusicChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(
                    connectOnMediaWhenAccessoryMusic = enabled
                )
            }
        },
    )
}
@Composable
fun ProxConnectSettingsScreen(
    navigateBack: (() -> Unit)?,

    alwaysConnectWhenAccessoryDisconnected: Boolean,
    onAlwaysConnectWhenAccessoryDisconnectedChanged: (Boolean) -> Unit,

    connectOnPhoneWhenAccessoryDisconnected: Boolean,
    onConnectOnPhoneWhenAccessoryDisconnectedChanged: (Boolean) -> Unit,
    connectOnPhoneWhenAccessoryIdle: Boolean,
    onConnectOnPhoneWhenAccessoryIdleChanged: (Boolean) -> Unit,
    connectOnPhoneWhenAccessoryMusic: Boolean,
    onConnectOnPhoneWhenAccessoryMusicChanged: (Boolean) -> Unit,

    connectOnMediaWhenAccessoryDisconnected: Boolean,
    onConnectOnMediaWhenAccessoryDisconnectedChanged: (Boolean) -> Unit,
    connectOnMediaWhenAccessoryIdle: Boolean,
    onConnectOnMediaWhenAccessoryIdleChanged: (Boolean) -> Unit,
    connectOnMediaWhenAccessoryMusic: Boolean,
    onConnectOnMediaWhenAccessoryMusicChanged: (Boolean) -> Unit,
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
                label = stringResource(R.string.always_connect_when_disconnected),
                description = stringResource(R.string.always_connect_when_disconnected_description),
                checked = alwaysConnectWhenAccessoryDisconnected,
                onCheckedChange = onAlwaysConnectWhenAccessoryDisconnectedChanged
            )

            StyledList(
                title = stringResource(R.string.prox_connect_on_phone),
                description = stringResource(R.string.prox_connect_on_phone_description)
            ) {
                styledListItem(
                    contentText = stringResource(R.string.prox_disconnected_label),
                    selected = connectOnPhoneWhenAccessoryDisconnected,
                    onClick = {
                        onConnectOnPhoneWhenAccessoryDisconnectedChanged(!connectOnPhoneWhenAccessoryDisconnected)
                    }
                )
                styledListItem(
                    contentText = stringResource(R.string.prox_idle_label),
                    selected = connectOnPhoneWhenAccessoryIdle,
                    onClick = {
                        onConnectOnPhoneWhenAccessoryIdleChanged(!connectOnPhoneWhenAccessoryIdle)
                    }
                )
                styledListItem(
                    contentText = stringResource(R.string.prox_music_label),
                    selected = connectOnPhoneWhenAccessoryMusic,
                    onClick = {
                        onConnectOnPhoneWhenAccessoryMusicChanged(!connectOnPhoneWhenAccessoryMusic)
                    }
                )
            }

            StyledList(
                title = stringResource(R.string.prox_connect_on_media),
                description = stringResource(R.string.prox_connect_on_media_description)
            ) {
                styledListItem(
                    contentText = stringResource(R.string.prox_disconnected_label),
                    selected = connectOnMediaWhenAccessoryDisconnected,
                    onClick = {
                        onConnectOnMediaWhenAccessoryDisconnectedChanged(!connectOnMediaWhenAccessoryDisconnected)
                    }
                )
                styledListItem(
                    contentText = stringResource(R.string.prox_idle_label),
                    selected = connectOnMediaWhenAccessoryIdle,
                    onClick = {
                        onConnectOnMediaWhenAccessoryIdleChanged(!connectOnMediaWhenAccessoryIdle)
                    }
                )
                styledListItem(
                    contentText = stringResource(R.string.prox_music_label),
                    selected = connectOnMediaWhenAccessoryMusic,
                    onClick = {
                        onConnectOnMediaWhenAccessoryMusicChanged(!connectOnMediaWhenAccessoryMusic)
                    }
                )
            }

            Spacer(modifier = Modifier.height(bottomPadding))
        }
    }
}
