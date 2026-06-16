package nz.eloque.heatr.ui.heater

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import nz.eloque.heatr.R
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
    val durations = stringArrayResource(R.array.durations).toList()
    val generations = stringArrayResource(R.array.generations).toList()
    val skins = stringArrayResource(R.array.skin_sensitivities).toList()

    var durationIndex by remember { mutableIntStateOf(0) }
    var generationIndex by remember { mutableIntStateOf(0) }
    var skinIndex by remember { mutableIntStateOf(0) }

    val startEnabled =
        when (state) {
            is HeatingViewModel.State.DeviceReady, HeatingViewModel.State.Done -> true
            is HeatingViewModel.State.Error -> hasDevice
            else -> false
        }

    val progressText =
        when (state) {
            is HeatingViewModel.State.Heating -> {
                val phaseName =
                    when (state.phase) {
                        HeatingPhase.HEATING -> "Heating"
                        HeatingPhase.APPLYING -> "Applying"
                        HeatingPhase.DONE -> "Done"
                    }
                "$phaseName — temp: ${state.temperature / 10} °C"
            }

            is HeatingViewModel.State.Done -> {
                "Cycle complete"
            }

            else -> {
                ""
            }
        }

    Scaffold { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .padding(16.dp),
        ) {
            Text(text = statusText, fontSize = 16.sp)

            Spacer(Modifier.height(24.dp))

            DropdownField(
                label = "Duration",
                options = durations,
                selectedIndex = durationIndex,
                onSelect = { durationIndex = it },
            )
            Spacer(Modifier.height(16.dp))
            DropdownField(
                label = "Generation",
                options = generations,
                selectedIndex = generationIndex,
                onSelect = { generationIndex = it },
            )
            Spacer(Modifier.height(16.dp))
            DropdownField(
                label = "Skin sensitivity",
                options = skins,
                selectedIndex = skinIndex,
                onSelect = { skinIndex = it },
            )

            Spacer(Modifier.height(24.dp))

            Button(
                onClick = {
                    onStart(
                        Duration.entries[durationIndex],
                        Generation.entries[generationIndex],
                        SkinSensitivity.entries[skinIndex],
                    )
                },
                enabled = startEnabled,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Start")
            }

            if (progressText.isNotEmpty()) {
                Spacer(Modifier.height(16.dp))
                Text(text = progressText, fontSize = 14.sp)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DropdownField(
    label: String,
    options: List<String>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }

    Text(text = label)
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
        OutlinedTextField(
            value = options[selectedIndex],
            onValueChange = {},
            readOnly = true,
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier =
                Modifier
                    .fillMaxWidth()
                    .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        onSelect(index)
                        expanded = false
                    },
                )
            }
        }
    }
}
