package me.kavishdevar.librepods.presentation.screens.settings.proxcard

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import me.kavishdevar.librepods.LibrePodsApplication
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.core.devices.AirPodsModel
import me.kavishdevar.librepods.core.devices.AirPodsSpecs
import me.kavishdevar.librepods.core.devices.Battery
import me.kavishdevar.librepods.core.devices.BatteryComponent
import me.kavishdevar.librepods.core.devices.BatterySource
import me.kavishdevar.librepods.core.devices.BatteryStatus
import me.kavishdevar.librepods.data.app.Design
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.data.app.FontFamilyOption
import me.kavishdevar.librepods.data.app.FontSettings
import me.kavishdevar.librepods.data.app.ProximityCardSettings
import me.kavishdevar.librepods.presentation.components.primitives.MaterialButtonStyle
import me.kavishdevar.librepods.presentation.components.primitives.StyledButton
import me.kavishdevar.librepods.presentation.components.primitives.StyledScaffold
import me.kavishdevar.librepods.presentation.components.primitives.StyledSlider
import me.kavishdevar.librepods.presentation.design.LibrePodsTheme
import me.kavishdevar.librepods.presentation.utils.LocalDebugMode
import me.kavishdevar.librepods.presentation.viewmodel.AppSettingsViewModel

@Composable
fun ProximityCardSizeAndPositionRoute(
    viewModel: AppSettingsViewModel,
    navigateBack: (() -> Unit)?
) {
    val uiState by viewModel.uiState.collectAsState()
    val settings = uiState.settings

    val context = LocalContext.current
    val proximityCardWindow = (context.applicationContext as LibrePodsApplication).proximityCardWindow

    val currentSettings = settings.proximityCardSettings

    DisposableEffect(Unit) {
        proximityCardWindow.show(
            name = "[Test] AirPods Pro",
            batteries = setOf(
                Battery(
                    BatteryComponent.LEFT,
                    70,
                    BatteryStatus.CHARGING,
                    BatterySource.AACP,
                    System.currentTimeMillis()
                ),
                Battery(
                    BatteryComponent.RIGHT,
                    68,
                    BatteryStatus.NOT_CHARGING,
                    BatterySource.AACP,
                    System.currentTimeMillis()
                ),
                Battery(
                    BatteryComponent.CASE,
                    20,
                    BatteryStatus.CHARGING,
                    BatterySource.AACP,
                    System.currentTimeMillis()
                )
            ),
            spec = AirPodsSpecs.getSpec(AirPodsModel.AIRPODS_PRO_3)
        )
        onDispose {
            proximityCardWindow.close()
        }
    }

    ProximityCardSizeAndPositionScreen(
        navigateBack = navigateBack,
        proximityCardSettings = settings.proximityCardSettings,
        updateProximityCardSettings = { proximityCardSettings ->
            viewModel.updateSettings {
                it.copy(
                    proximityCardSettings = proximityCardSettings
                )
            }
        },
        resetSettings = {
            viewModel.updateSettings {
                it.copy(
                    proximityCardSettings = currentSettings
                )
            }
        },
        showProximityCard = {
            proximityCardWindow.show()
        },
        hideProximityCard = {
            proximityCardWindow.close()
        }
    )
}

@Composable
fun ProximityCardSizeAndPositionScreen(
    navigateBack: (() -> Unit)? = null,
    proximityCardSettings: ProximityCardSettings,
    updateProximityCardSettings: (ProximityCardSettings) -> Unit,
    resetSettings: () -> Unit,
    showProximityCard: () -> Unit,
    hideProximityCard: () -> Unit
) {
    val scrollState = rememberScrollState()

    val debugMode = LocalDebugMode.current

    val containerSize = LocalWindowInfo.current.containerSize
    val screenWidth = containerSize.width
    val screenHeight = containerSize.height

    StyledScaffold(
        title = stringResource(R.string.size_and_position),
        navigateBack = navigateBack
    ) { topPadding, bottomPadding ->
        Column(
            modifier = Modifier
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Spacer(modifier = Modifier.height(topPadding))

            Column(
                modifier = Modifier
                    .weight(1f)
                    .verticalScroll(scrollState),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                StyledSlider(
                    label = stringResource(R.string.vertical_offset) + if (debugMode) " (${proximityCardSettings.offsetY})" else "",
                    value = proximityCardSettings.offsetY,
                    onValueChange = {
                        updateProximityCardSettings(
                            proximityCardSettings.copy(
                                offsetY = it
                            )
                        )
                    },
                    valueRange = 0f..(screenHeight / 4f),
                    independent = true
                )

                StyledSlider(
                    label = stringResource(R.string.height) + if (debugMode) " (${proximityCardSettings.height})" else "",
                    value = proximityCardSettings.height,
                    onValueChange = {
                        updateProximityCardSettings(
                            proximityCardSettings.copy(
                                height = it
                            )
                        )
                    },
                    valueRange = 0f..(screenHeight / 2f),
                    independent = true
                )

                StyledSlider(
                    label = stringResource(R.string.width) + if (debugMode) " (${proximityCardSettings.width})" else "",
                    value = proximityCardSettings.width,
                    onValueChange = {
                        updateProximityCardSettings(
                            proximityCardSettings.copy(
                                width = it
                            )
                        )
                    },
                    valueRange = 0f..screenWidth.toFloat(),
                    independent = true
                )

                StyledButton(
                    modifier = Modifier.fillMaxWidth(),
                    onClick = { resetSettings() },
                    materialButtonStyle = MaterialButtonStyle.Outlined
                ) {
                    Text(
                        text = stringResource(R.string.reset),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                StyledButton(
                    modifier = Modifier.weight(1f),
                    onClick = {
                        hideProximityCard()
                    },
                    materialButtonStyle = MaterialButtonStyle.Outlined
                ) {
                    Text(
                        text = stringResource(R.string.hide_prox_card),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                StyledButton(
                    modifier = Modifier.weight(1f),
                    onClick = {
                        showProximityCard()
                    },
                    materialButtonStyle = MaterialButtonStyle.Outlined
                ) {
                    Text(
                        text = stringResource(R.string.show_prox_card),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }

            Spacer(modifier = Modifier.height(bottomPadding))
        }
    }
}

@Preview
@Composable
fun ProximityCardSizeAndPositionScreenPreview() {
    LibrePodsTheme(
//        designSystem = DesignSystem.Apple,
//        fontSettings = FontSettings(fontFamilyOption = FontFamilyOption.Inter),
        design = Design(
            system = DesignSystem.Material
        ),
        fontSettings = FontSettings(fontFamilyOption = FontFamilyOption.RobotoFlexExpressive),
        darkTheme = true
    ) {
        val proximityCardSettings = remember { mutableStateOf(ProximityCardSettings()) }
        ProximityCardSizeAndPositionScreen(
            proximityCardSettings = proximityCardSettings.value,
            updateProximityCardSettings = { proximityCardSettings.value = it },
            resetSettings = {},
            showProximityCard = {},
            hideProximityCard = {}
        )
    }
}
