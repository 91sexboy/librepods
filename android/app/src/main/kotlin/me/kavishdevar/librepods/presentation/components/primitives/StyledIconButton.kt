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

package me.kavishdevar.librepods.presentation.components.primitives

import android.content.res.Configuration.UI_MODE_NIGHT_NO
import android.content.res.Configuration.UI_MODE_NIGHT_YES
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.BiasAlignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.util.fastCoerceAtMost
import androidx.compose.ui.util.lerp
import dev.chrisbanes.haze.HazeInput
import dev.chrisbanes.haze.glass.GlassStyle
import dev.chrisbanes.haze.glass.hazeGlass
import kotlinx.coroutines.launch
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.presentation.design.LocalDesign
import me.kavishdevar.librepods.presentation.icons.LocalIcons
import me.kavishdevar.librepods.utils.inspectDragGestures
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.tanh

@Composable
fun StyledIconButton(
    modifier: Modifier = Modifier,
    surfaceColor: Color = Color.Unspecified,
    hazeInput: HazeInput = HazeInput.Content,
    onClick: () -> Unit,
    enabled: Boolean = true,
    materialButtonStyle: MaterialButtonStyle = MaterialButtonStyle.Normal,
    content: @Composable () -> Unit,
) {
    val design = LocalDesign.current

    when (design.system) {
        DesignSystem.Material -> {
            when (materialButtonStyle) {
                MaterialButtonStyle.Tonal -> {
                    FilledTonalIconButton(
                        onClick = onClick,
                        enabled = enabled,
                        modifier = Modifier.size(52.dp)
                    ) {
                        content()
                    }
                }
                MaterialButtonStyle.Filled -> {
                    FilledIconButton(
                        onClick = onClick,
                        enabled = enabled,
                        modifier = Modifier.size(52.dp)
                    ) {
                        content()
                    }
                }
                MaterialButtonStyle.Outlined -> {
                    OutlinedIconButton(
                        onClick = onClick,
                        enabled = enabled,
                        modifier = Modifier.size(52.dp)
                    ) {
                        content()
                    }
                }
                MaterialButtonStyle.Normal -> {
                    IconButton(
                        onClick = onClick,
                        enabled = enabled,
                        modifier = Modifier.size(52.dp)
                    ) {
                        content()
                    }
                }
            }
        }
        DesignSystem.Apple -> {
            val haptics = LocalHapticFeedback.current
            val scope = rememberCoroutineScope()
            val progressAnimationSpec = spring(0.5f, 300f, 0.001f)
            val offsetAnimationSpec = spring(1f, 300f, Offset.VisibilityThreshold)
            val progressAnimation = remember { Animatable(0f) }
            val offsetAnimation = remember { Animatable(Offset.Zero, Offset.VectorConverter) }
            var pressStartPosition by remember { mutableStateOf(Offset.Zero) }

            val isDarkTheme = MaterialTheme.colorScheme.surfaceContainer.luminance() < 0.5f

            val density = LocalDensity.current

            Box(
                contentAlignment = Alignment.Center,
                modifier = modifier
                    .padding(horizontal = 12.dp)
                    .clickable(
                        interactionSource = null,
                        indication = null
                    ) {
                        if (enabled) {
                            scope.launch { haptics.performHapticFeedback(HapticFeedbackType.ContextClick) }
                            onClick()
                        }
                    }
                    .graphicsLayer {
                        val progress = progressAnimation.value
                        val scale = lerp(1f, 1.5f, progress)

                        val maxOffset = size.minDimension
                        val initialDerivative = 0.05f
                        val offset = offsetAnimation.value
                        translationX = maxOffset * tanh(initialDerivative * offset.x / maxOffset)
                        translationY = maxOffset * tanh(initialDerivative * offset.y / maxOffset)

                        val width = size.width
                        val height = size.height
                        val maxDragScale = 0.1f
                        val offsetAngle = atan2(offset.y, offset.x)

                        scaleX = scale + maxDragScale * abs(cos(offsetAngle) * offset.x / size.maxDimension) *
                            (width / height).fastCoerceAtMost(1f)
                        scaleY = scale + maxDragScale * abs(sin(offsetAngle) * offset.y / size.maxDimension) *
                            (height / width).fastCoerceAtMost(1f)
                    }
                    .hazeGlass(
                        input = hazeInput,
                        style = GlassStyle.regular.then {
                            shape(CircleShape)
                            lightPosition(BiasAlignment(horizontalBias = 1.2f, verticalBias = -1.5f))
                            edgeShadow(Color.Black.copy(alpha = 0.2f))
                            edgeSoftness(1.5.dp)
                            tint(
                                if (isDarkTheme) Color.DarkGray.copy(0.4f) else Color.LightGray.copy(0.1f)
                            )
                        }
                    )
                    .pointerInput(scope) {
                        val onDragStop: () -> Unit = {
                            if (enabled) {
                                scope.launch {
                                    launch { haptics.performHapticFeedback(HapticFeedbackType.Reject) }
                                    launch {
                                        progressAnimation.animateTo(
                                            0f,
                                            progressAnimationSpec
                                        )
                                    }
                                    launch {
                                        offsetAnimation.animateTo(
                                            Offset.Zero,
                                            offsetAnimationSpec
                                        )
                                    }
                                }
                            }
                        }
                        inspectDragGestures(
                            onDragStart = { down ->
                                if (enabled) {
                                    pressStartPosition = down.position
                                    scope.launch {
                                        launch { haptics.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick) }
                                        launch {
                                            progressAnimation.animateTo(
                                                1f,
                                                progressAnimationSpec
                                            )
                                        }
                                        launch { offsetAnimation.snapTo(Offset.Zero) }
                                    }
                                }
                            },
                            onDragEnd = { onDragStop() },
                            onDragCancel = onDragStop
                        ) { _, dragAmount ->
                            scope.launch {
                                if (enabled) {
                                    if (dragAmount.getDistanceSquared() > 350) {
                                        haptics.performHapticFeedback(
                                            HapticFeedbackType.SegmentFrequentTick
                                        )
                                    }
                                    offsetAnimation.snapTo(offsetAnimation.value + dragAmount)
                                }
                            }
                        }
                    }
                    .size(with(density) { 48.sp.toDp() }),
            ) {
                Box(
                    modifier = Modifier.padding(8.dp)
                ) {
                    content()
                }
            }
        }
    }
}

@Preview(uiMode = UI_MODE_NIGHT_NO, name = "Light")
@Preview(uiMode = UI_MODE_NIGHT_YES, name = "Dark")
@Composable
fun StyledIconButtonPreview() {
    Box(modifier = Modifier
        .height(120.dp)
        .width(200.dp)
        .background(
            MaterialTheme.colorScheme.surfaceContainer,
            RoundedCornerShape(28.dp)
        ), contentAlignment = Alignment.Center) {
        StyledIconButton(
            onClick = { }
        ) {
            Icon(
                imageVector = LocalIcons.current.Settings,
                contentDescription = "Settings",
            )
        }
    }
}
