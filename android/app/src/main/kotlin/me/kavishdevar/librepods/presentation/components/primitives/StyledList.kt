package me.kavishdevar.librepods.presentation.components.primitives

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.isSpecified
import androidx.compose.ui.tooling.preview.AndroidUiModes.UI_MODE_NIGHT_YES
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.Wallpapers.GREEN_DOMINATED_EXAMPLE
import androidx.compose.ui.unit.dp
import me.kavishdevar.librepods.data.app.ColorStyle
import me.kavishdevar.librepods.data.app.Design
import me.kavishdevar.librepods.data.app.DesignSystem
import me.kavishdevar.librepods.presentation.design.LibrePodsTheme
import me.kavishdevar.librepods.presentation.design.LocalDesign
import me.kavishdevar.librepods.presentation.design.sectionHeader

@Composable
fun StyledList(
    modifier: Modifier = Modifier,
    scrollEnabled: Boolean = false,
    title: String? = null,
    description: String? = null,
    key: Any? = null,
    surfaceColor: Color = Color.Unspecified,
    content: @Composable StyledListScope.() -> Unit
) {
    val scope = StyledListScope()
    key(key) {
        scope.content()
    }

    val design = LocalDesign.current
    val m3eEnabled = design.system == DesignSystem.Material

    Column {
        title?.let {
            Box(
                modifier = Modifier
                    .background(if (surfaceColor.isSpecified) surfaceColor else if (design.colorStyle == ColorStyle.Custom) MaterialTheme.colorScheme.surface else MaterialTheme.colorScheme.surfaceContainer)
                    .padding(horizontal = 16.dp)
                    .padding(top = 4.dp, bottom = if (m3eEnabled) 12.dp else 4.dp)
            ) {
                Text(
                    text = it,
                    color = if (m3eEnabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.sectionHeader,
                    style = MaterialTheme.typography.labelMediumEmphasized
                )
            }
        }
        val scrollState = rememberScrollState()
        Column(
            modifier = modifier
                .fillMaxWidth()
                .animateContentSize(MaterialTheme.motionScheme.defaultSpatialSpec())
                .background(
                    color = if (m3eEnabled) Color.Transparent else MaterialTheme.colorScheme.surfaceContainerHigh,
                    shape = RoundedCornerShape(if (m3eEnabled) 24.dp else 28.dp)
                )
                .clip(RoundedCornerShape(if (m3eEnabled) 24.dp else 28.dp))
                .then(if (scrollEnabled) Modifier.verticalScroll(scrollState) else Modifier)
        ) {
            if (m3eEnabled && description != null) {
                Text(
                    text = description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onBackground.copy(0.8f),
                    modifier = Modifier.padding(horizontal = 16.dp)
                )
                Spacer(modifier = Modifier.height(8.dp))
            }
            scope.items.forEachIndexed { index, item ->
                item(index, scope.items.size)
            }
            Spacer(modifier = Modifier.height(if(m3eEnabled) 4.dp else 0.dp))
        }
        if (!m3eEnabled && description != null) {
            Box(
                modifier = Modifier
                    .background(if (surfaceColor.isSpecified) surfaceColor else if (design.colorStyle == ColorStyle.Custom) MaterialTheme.colorScheme.surface else MaterialTheme.colorScheme.surfaceContainer)
                    .padding(vertical = 8.dp, horizontal = 16.dp)
            ) {
                Text(
                    text = description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onBackground.copy(0.6f),
                )
            }
        }
    }
}

class StyledListScope {
    internal val items =
        mutableListOf<@Composable (Int, Int) -> Unit>()

    fun item(
        content: @Composable (index: Int, count: Int) -> Unit
    ) {
        items += content
    }
}

@Preview(showBackground = true, wallpaper = GREEN_DOMINATED_EXAMPLE, uiMode = UI_MODE_NIGHT_YES)
@Composable
fun StyledListDemo() {
    LibrePodsTheme(
        design = Design(
            system = DesignSystem.Apple
        ),
        darkTheme = false
    ) {
        StyledScaffold(
            title = "StyledListTest",
            navigateBack = null
        ) { topPadding, bottomPadding ->
            Column (
                modifier = Modifier.padding(horizontal = 12.dp)
            ) {
                Spacer(modifier = Modifier.height(topPadding))
                StyledList(
                    title = "hello"
                ) {
                    for (i in 0..2) {
                        styledListItem(
                            contentText = i.toString(),
                            onClick = {}
                        )
                    }
                    val checked = remember { mutableStateOf(false) }
                    styledToggle(
                        label = "Test",
                        description = "Lorem ipsum dolor sit amet, consectetur adipiscing elit mollit anim id est laborum.",
                        checked = checked.value,
                        onCheckedChange = { checked.value = it },
                    )
                }
                Spacer(modifier = Modifier.height(bottomPadding))
            }
        }
    }
}
