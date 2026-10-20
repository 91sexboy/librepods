package me.kavishdevar.librepods.presentation.screens.settings.appearance

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.core.graphics.ColorUtils
import com.materialkolor.PaletteStyle
import dev.chrisbanes.haze.HazeInput
import dev.chrisbanes.haze.glass.GlassStyle
import dev.chrisbanes.haze.glass.hazeGlass
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState
import kotlinx.coroutines.flow.debounce
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.data.app.ColorStyle
import me.kavishdevar.librepods.data.app.Design
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.presentation.components.primitives.StyledList
import me.kavishdevar.librepods.presentation.components.primitives.StyledListItem
import me.kavishdevar.librepods.presentation.components.primitives.StyledScaffold
import me.kavishdevar.librepods.presentation.components.primitives.StyledSlider
import me.kavishdevar.librepods.presentation.components.primitives.StyledToggle
import me.kavishdevar.librepods.presentation.components.primitives.styledListItem
import me.kavishdevar.librepods.presentation.design.LibrePodsTheme
import me.kavishdevar.librepods.presentation.design.LocalDesign
import me.kavishdevar.librepods.presentation.viewmodel.AppSettingsViewModel
import kotlin.time.Duration.Companion.milliseconds

@Composable
fun AppColorSettingsRoute(
    viewModel: AppSettingsViewModel,
    navigateBack: (() -> Unit)?
) {
    val uiState by viewModel.uiState.collectAsState()
    val settings = uiState.settings

    AppColorSettingsScreen(
        navigateBack = navigateBack,
        colorStyle = settings.colorStyle,
        onColorStyleChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(colorStyle = enabled)
            }
        },
        selectedColor = settings.overrideMaterialColor,
        onColorChanged = { newColor ->
            viewModel.updateSettings {
                it.copy(overrideMaterialColor = newColor)
            }
        },
        paletteStyle = settings.paletteStyle,
        onPaletteStyleChanged = { style ->
            viewModel.updateSettings {
                it.copy(paletteStyle = style)
            }
        },
        contrastValue = settings.contrastLevel.toFloat(),
        onContrastValueChange = { value ->
            viewModel.updateSettings {
                it.copy(contrastLevel = value.toDouble())
            }
        },
        amoledMode = settings.amoledMode,
        onAmoledModeChanged = { enabled ->
            viewModel.updateSettings {
                it.copy(amoledMode = enabled)
            }
        }
    )
}

@Composable
fun AppColorSettingsScreen(
    navigateBack: (() -> Unit)? = null,
    colorStyle: ColorStyle,
    onColorStyleChanged: (ColorStyle) -> Unit,
    selectedColor: Color,
    onColorChanged: (Color) -> Unit,
    paletteStyle: PaletteStyle,
    onPaletteStyleChanged: (PaletteStyle) -> Unit,
    contrastValue: Float,
    onContrastValueChange: (Float) -> Unit,
    amoledMode: Boolean,
    onAmoledModeChanged: (Boolean) -> Unit
) {
    val scrollState = rememberScrollState()

    val initialHsl = FloatArray(3).apply {
        ColorUtils.colorToHSL(selectedColor.toArgb(), this)
    }

    var hue by remember { mutableFloatStateOf(initialHsl[0]) }
    var saturation by remember { mutableFloatStateOf(initialHsl[1]) }
    var lightness by remember { mutableFloatStateOf(initialHsl[2]) }

    LaunchedEffect(hue, saturation, lightness) {
        snapshotFlow { Triple(hue, saturation, lightness) }
            .debounce(150.milliseconds)
            .collect { (h, s, l) ->
                onColorChanged(
                    Color.hsl(
                        hue = h.coerceIn(0f, 360f),
                        saturation = s.coerceIn(0f, 1f),
                        lightness = l.coerceIn(0f, 1f)
                    )
                )
            }
    }

    val hueColors = remember {
        listOf(
            Color.hsl(0f, 1f, 0.5f),
            Color.hsl(60f, 1f, 0.5f),
            Color.hsl(120f, 1f, 0.5f),
            Color.hsl(180f, 1f, 0.5f),
            Color.hsl(240f, 1f, 0.5f),
            Color.hsl(300f, 1f, 0.5f),
            Color.hsl(360f, 1f, 0.5f)
        )
    }

    val saturationColors = remember(hue, lightness) {
        val safeL = lightness.coerceIn(0.18f, 0.82f)
        listOf(
            Color.hsl(hue, 0f, safeL),
            Color.hsl(hue, 1f, safeL)
        )
    }

    val lightnessColors = remember(hue, saturation) {
        listOf(
            Color(0xFF0A0A0C),
            Color.hsl(hue, saturation, 0.5f),
            Color(0xFFF8F9FA)
        )
    }

    val contrastSliderValue = remember { mutableFloatStateOf(contrastValue) }

    LaunchedEffect(
        contrastSliderValue.floatValue
    ) {
        snapshotFlow { contrastSliderValue.floatValue }
            .debounce(150.milliseconds)
            .collect {
                onContrastValueChange(it)
            }
    }

    StyledScaffold(
        title = stringResource(R.string.app_color),
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
                    contentText = stringResource(R.string.default_option),
                    selected = colorStyle == ColorStyle.Default,
                    onClick = { onColorStyleChanged(ColorStyle.Default) }
                )
                styledListItem(
                    contentText = stringResource(R.string.wallpaper),
                    selected = colorStyle == ColorStyle.Wallpaper,
                    onClick = { onColorStyleChanged(ColorStyle.Wallpaper) }
                )
                styledListItem(
                    contentText = stringResource(R.string.custom),
                    selected = colorStyle == ColorStyle.Custom,
                    onClick = { onColorStyleChanged(ColorStyle.Custom) }
                )
            }

            AnimatedVisibility(visible = colorStyle == ColorStyle.Custom) {
                Column(
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    StyledToggle(
                        label = stringResource(R.string.black_background),
                        checked = amoledMode,
                        onCheckedChange = onAmoledModeChanged
                    )

                    val showPaletteStyles = remember { mutableStateOf(false) }

                    StyledList(key = showPaletteStyles.value) {
                        styledListItem(
                            contentText = stringResource(R.string.palette_style),
                            supportingText = paletteStyle.name,
                            onClick = { showPaletteStyles.value = !showPaletteStyles.value }
                        )

                        if (showPaletteStyles.value) {
                            styledListItem(
                                contentText = PaletteStyle.TonalSpot.name,
                                selected = paletteStyle == PaletteStyle.TonalSpot,
                                onClick = { onPaletteStyleChanged(PaletteStyle.TonalSpot) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Neutral.name,
                                selected = paletteStyle == PaletteStyle.Neutral,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Neutral) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Vibrant.name,
                                selected = paletteStyle == PaletteStyle.Vibrant,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Vibrant) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Expressive.name,
                                selected = paletteStyle == PaletteStyle.Expressive,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Expressive) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Rainbow.name,
                                selected = paletteStyle == PaletteStyle.Rainbow,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Rainbow) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.FruitSalad.name,
                                selected = paletteStyle == PaletteStyle.FruitSalad,
                                onClick = { onPaletteStyleChanged(PaletteStyle.FruitSalad) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Monochrome.name,
                                selected = paletteStyle == PaletteStyle.Monochrome,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Monochrome) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Fidelity.name,
                                selected = paletteStyle == PaletteStyle.Fidelity,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Fidelity) }
                            )
                            styledListItem(
                                contentText = PaletteStyle.Content.name,
                                selected = paletteStyle == PaletteStyle.Content,
                                onClick = { onPaletteStyleChanged(PaletteStyle.Content) }
                            )
                        }
                    }

                    if (LocalDesign.current.system == DesignSystem.Material) {
                        StyledList(title = "HSL Channels") {
                            styledListItem(
                                content = {
                                    HSLSliderRow(
                                        label = "Hue",
                                        valueText = "${hue.toInt()}°",
                                        value = hue,
                                        valueRange = 0f..360f,
                                        gradientColors = hueColors,
                                        onValueChange = {
                                            hue = it
                                        }
                                    )
                                }
                            )

                            styledListItem(
                                content = {
                                    HSLSliderRow(
                                        label = "Saturation",
                                        valueText = "${(saturation * 100).toInt()}%",
                                        value = saturation,
                                        valueRange = 0f..1f,
                                        gradientColors = saturationColors,
                                        onValueChange = {
                                            saturation = it
                                        }
                                    )
                                }
                            )

                            styledListItem(
                                content = {
                                    HSLSliderRow(
                                        label = "Lightness",
                                        valueText = "${(lightness * 100).toInt()}%",
                                        value = lightness,
                                        valueRange = 0f..1f,
                                        gradientColors = lightnessColors,
                                        onValueChange = {
                                            lightness = it
                                        }
                                    )
                                }
                            )
                        }
                    } else {
                        Column(
                            verticalArrangement = Arrangement.spacedBy(16.dp)
                        ) {
                            StyledListItem(
                                onClick = null,
                                title = "Hue",
                                content = @Composable {
                                    Box(
                                        modifier = Modifier.padding(vertical = 16.dp)
                                    ) {
                                        HSLSlider(
                                            value = hue,
                                            valueRange = 0f..360f,
                                            gradientColors = hueColors,
                                            onValueChange = {
                                                hue = it
                                            }
                                        )
                                    }
                                },
                            )

                            StyledListItem(
                                onClick = null,
                                title = "Saturation",
                                content = @Composable {
                                    Box(
                                        modifier = Modifier.padding(vertical = 16.dp)
                                    ) {
                                        HSLSlider(
                                            value = saturation,
                                            valueRange = 0f..1f,
                                            gradientColors = saturationColors,
                                            onValueChange = {
                                                saturation = it
                                            }
                                        )
                                    }
                                },
                            )

                            StyledListItem(
                                onClick = null,
                                title = "Lightness",
                                content = @Composable {
                                    Box(
                                        modifier = Modifier.padding(vertical = 16.dp)
                                    ) {
                                        HSLSlider(
                                            value = lightness,
                                            valueRange = 0f..1f,
                                            gradientColors = lightnessColors,
                                            onValueChange = {
                                                lightness = it
                                            }
                                        )
                                    }
                                },
                            )
                        }
                    }

                    StyledSlider(
                        value = contrastSliderValue.floatValue,
                        valueRange = -1f..1f,
                        onValueChange = {
                            contrastSliderValue.floatValue = it
                        },
                        independent = true,
                        label = "Contrast"
                    )
                }
            }

            Spacer(modifier = Modifier.height(bottomPadding))
        }
    }
}

@Composable
private fun HSLSliderRow(
    label: String,
    valueText: String,
    value: Float,
    valueRange: ClosedFloatingPointRange<Float>,
    gradientColors: List<Color>,
    onValueChange: (Float) -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 4.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium
            )
            Text(
                text = valueText,
                style = MaterialTheme.typography.bodySmall.copy(
                    fontFamily = FontFamily.Monospace
                ),
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }

        HSLSlider(
            value = value,
            valueRange = valueRange,
            gradientColors = gradientColors,
            onValueChange = onValueChange
        )
    }
}

@Composable
private fun HSLSlider(
    value: Float,
    valueRange: ClosedFloatingPointRange<Float>,
    gradientColors: List<Color>,
    onValueChange: (Float) -> Unit,
    modifier: Modifier = Modifier
) {
    val trackHazeState = rememberHazeState()
    var isDragging by remember { mutableStateOf(false) }

    val rangeSpan = valueRange.endInclusive - valueRange.start
    val fraction = ((value - valueRange.start) / rangeSpan).coerceIn(0f, 1f)

    val thumbWidth by animateDpAsState(
        targetValue = if (isDragging) 38.dp else 24.dp,
        animationSpec = spring(
            dampingRatio = Spring.DampingRatioMediumBouncy,
            stiffness = Spring.StiffnessMediumLow
        ),
        label = "thumbWidth"
    )
    val trackScale by animateFloatAsState(
        targetValue = if (isDragging) 1.015f else 1f,
        animationSpec = spring(
            dampingRatio = Spring.DampingRatioMediumBouncy,
            stiffness = Spring.StiffnessMedium
        ),
        label = "trackScale"
    )

    val trackHeight = 44.dp
    val trackInset = 4.dp
    val thumbHeight = trackHeight - (trackInset * 2)

    val thumbGlassStyle = remember(isDragging) {
        val base = if (isDragging) GlassStyle.clear else GlassStyle.regular
        base.then {
            tint(Color.White.copy(alpha = if (isDragging) 0.14f else 0.22f))
            shape(CircleShape)
        }
    }

    BoxWithConstraints(
        modifier = modifier
            .fillMaxWidth()
            .height(trackHeight)
            .graphicsLayer {
                scaleX = trackScale
                scaleY = trackScale
            }
            .clip(RoundedCornerShape(percent = 50))
            .pointerInput(valueRange) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    isDragging = true

                    fun dispatchFromX(x: Float) {
                        val paddingPx = trackInset.toPx()
                        val currentThumbWidthPx = thumbWidth.toPx()
                        val usableWidth =
                            (size.width - paddingPx * 2 - currentThumbWidthPx).coerceAtLeast(1f)
                        val adjustedX = x - paddingPx - currentThumbWidthPx / 2f
                        val newFraction = (adjustedX / usableWidth).coerceIn(0f, 1f)
                        onValueChange(valueRange.start + newFraction * rangeSpan)
                    }

                    dispatchFromX(down.position.x)

                    do {
                        val event = awaitPointerEvent()
                        val change = event.changes.firstOrNull()
                        if (change != null && change.pressed) {
                            change.consume()
                            dispatchFromX(change.position.x)
                        }
                    } while (event.changes.any { it.pressed })

                    isDragging = false
                }
            }
    ) {
        val usableTravelDistance = (maxWidth - (trackInset * 2) - thumbWidth).coerceAtLeast(0.dp)
        val thumbOffsetX = trackInset + (usableTravelDistance * fraction)

        Canvas(
            modifier = Modifier
                .fillMaxSize()
                .hazeSource(trackHazeState)
        ) {
            drawRect(brush = Brush.horizontalGradient(gradientColors))
        }

        Box(
            modifier = Modifier
                .offset { IntOffset(thumbOffsetX.roundToPx(), trackInset.roundToPx()) }
                .size(width = thumbWidth, height = thumbHeight)
                .clip(CircleShape)
                .hazeGlass(
                    input = HazeInput.Sources(trackHazeState),
                    style = thumbGlassStyle
                ),
            contentAlignment = Alignment.Center
        ) {
            val pillHeight by animateDpAsState(
                targetValue = if (isDragging) 6.dp else 14.dp,
                animationSpec = spring(stiffness = Spring.StiffnessMedium),
                label = "pillHeight"
            )
            Box(
                modifier = Modifier
                    .size(width = 4.dp, height = pillHeight)
                    .clip(CircleShape)
                    .background(Color.White.copy(alpha = 0.85f))
            )
        }
    }
}

@Preview
@Composable
fun AppColorSettingsScreenPreview() {
    val colorStyle = remember { mutableStateOf(ColorStyle.Default) }
    val color = remember { mutableStateOf(Color(0xFF00FF40)) }
    val paletteStyle = remember { mutableStateOf(PaletteStyle.parse("TonalSpot")) }
    val contrastValue = remember { mutableFloatStateOf(0.0f) }
    val amoledMode = remember { mutableStateOf(false) }

    LibrePodsTheme(
        design = Design(
            system = DesignSystem.Apple,
            colorStyle = colorStyle.value,
            overrideMaterialColor = color.value,
            paletteStyle = paletteStyle.value,
            contrastLevel = contrastValue.floatValue.toDouble()
        ),
        darkTheme = true
    ) {
        AppColorSettingsScreen(
            colorStyle = colorStyle.value,
            onColorStyleChanged = {
                colorStyle.value = it
            },
            selectedColor = color.value,
            onColorChanged = {
                color.value = it
            },
            paletteStyle = paletteStyle.value,
            onPaletteStyleChanged = {
                paletteStyle.value = it
            },
            contrastValue = contrastValue.floatValue,
            onContrastValueChange = {
                contrastValue.floatValue = it
            },
            amoledMode = amoledMode.value,
            onAmoledModeChanged = {
                amoledMode.value = it
            }
        )
    }
}
