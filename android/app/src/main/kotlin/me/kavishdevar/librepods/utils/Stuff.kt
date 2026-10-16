package me.kavishdevar.librepods.utils

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.sp

@Composable
fun Int.nonScaledSp() = (this / LocalDensity.current.fontScale).sp
