package nz.eloque.heatr.ui.heater

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AccessTime
import androidx.compose.material.icons.outlined.ChildCare
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material.icons.outlined.Spa
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults.elevatedCardColors
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import nz.eloque.heatr.api.Duration
import nz.eloque.heatr.api.Generation
import nz.eloque.heatr.api.HeatingPhase
import nz.eloque.heatr.api.SkinSensitivity

@Composable
fun HeatrScreen(
    statusText: String,
    state: HeatingViewModel.State,
    hasDevice: Boolean,
    onInit: () -> Unit,
    onStart: (Duration, Generation, SkinSensitivity) -> Unit,
) {
    var durationIndex by remember { mutableIntStateOf(0) }
    var generationIndex by remember { mutableIntStateOf(0) }
    var skinIndex by remember { mutableIntStateOf(0) }

    val startEnabled =
        when (state) {
            is HeatingViewModel.State.DeviceReady,
            HeatingViewModel.State.Done,
            -> true

            is HeatingViewModel.State.Error -> hasDevice

            else -> false
        }

    val progressText =
        when (state) {
            is HeatingViewModel.State.Heating -> {
                val phase =
                    when (state.phase) {
                        HeatingPhase.HEATING -> "Heating"
                        HeatingPhase.APPLYING -> "Applying"
                        HeatingPhase.DONE -> "Done"
                    }

                "$phase • ${state.temperature}°C"
            }

            HeatingViewModel.State.Done -> {
                "Cycle complete"
            }

            else -> {
                ""
            }
        }

    LaunchedEffect(Unit) {
        onInit()
    }

    LazyColumn(
        modifier =
            Modifier
                .fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        item {
            Text(
                text = statusText,
                style = MaterialTheme.typography.bodyLarge,
            )
        }

        item {
            SectionTitle("Duration")
        }

        item {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Duration.entries.forEachIndexed { index, duration ->

                    SelectionCard(
                        modifier = Modifier.weight(1f),
                        title =
                            duration.name.lowercase()
                                .replaceFirstChar { it.uppercase() },
                        icon = {
                            Icon(
                                Icons.Outlined.AccessTime,
                                null,
                                modifier = Modifier.size(28.dp),
                            )
                        },
                        selected = durationIndex == index,
                        onClick = {
                            durationIndex = index
                        },
                    )
                }
            }
        }

        item {
            SectionTitle("Generation")
        }

        item {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                SelectionCard(
                    modifier = Modifier.weight(1f),
                    title = "Child",
                    icon = {
                        Icon(
                            Icons.Outlined.ChildCare,
                            null,
                            modifier = Modifier.size(40.dp),
                        )
                    },
                    selected = generationIndex == 0,
                    onClick = {
                        generationIndex = 0
                    },
                )

                SelectionCard(
                    modifier = Modifier.weight(1f),
                    title = "Adult",
                    icon = {
                        Icon(
                            Icons.Outlined.Person,
                            null,
                            modifier = Modifier.size(40.dp),
                        )
                    },
                    selected = generationIndex == 1,
                    onClick = {
                        generationIndex = 1
                    },
                )
            }
        }

        item {
            SectionTitle("Skin sensitivity")
        }

        item {
            ElevatedCard(
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(20.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Outlined.Spa,
                        null,
                    )

                    Spacer(Modifier.size(16.dp))

                    Text(
                        "Sensitive",
                        modifier = Modifier.weight(1f),
                    )

                    Switch(
                        checked = skinIndex == 1,
                        onCheckedChange = {
                            skinIndex = if (it) 1 else 0
                        },
                    )
                }
            }
        }

        item {
            Spacer(Modifier.height(8.dp))

            Button(
                onClick = {
                    onStart(
                        Duration.entries[durationIndex],
                        Generation.entries[generationIndex],
                        SkinSensitivity.entries[skinIndex],
                    )
                },
                enabled = startEnabled,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(60.dp),
            ) {
                Text("Start")
            }
        }

        if (progressText.isNotEmpty()) {
            item {
                Text(
                    progressText,
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
    }
}

@Composable
private fun SectionTitle(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleMedium,
        fontWeight = FontWeight.SemiBold,
    )
}

@Composable
private fun SelectionCard(
    modifier: Modifier = Modifier,
    title: String,
    icon: @Composable () -> Unit,
    selected: Boolean,
    onClick: () -> Unit,
) {
    ElevatedCard(
        onClick = onClick,
        modifier =
            modifier.height(120.dp).apply {
                if (selected) {
                    this.border(
                        BorderStroke(
                            2.dp,
                            MaterialTheme.colorScheme.primary,
                        ),
                    )
                }
            },
        colors =
            elevatedCardColors(
                containerColor =
                    if (selected) {
                        MaterialTheme.colorScheme.primaryContainer
                    } else {
                        MaterialTheme.colorScheme.surfaceContainer
                    },
            ),
    ) {
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(12.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            icon()

            Spacer(Modifier.height(12.dp))

            Text(
                title,
                style = MaterialTheme.typography.bodyMedium,
            )
        }
    }
}
