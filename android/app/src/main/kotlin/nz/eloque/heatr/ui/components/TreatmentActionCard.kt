package nz.eloque.heatr.ui.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults.elevatedCardColors
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import nz.eloque.heatr.R
import nz.eloque.heatr.api.HeatingPhase
import nz.eloque.heatr.ui.heater.HeatingViewModel

@Composable
fun TreatmentActionCard(
    state: HeatingViewModel.State,
    enabled: Boolean,
    temperature: Int?,
    onStart: () -> Unit,
) {
    val (title, color, clickable) =
        when (state) {
            is HeatingViewModel.State.Heating -> {
                val phaseText =
                    when (state.phase) {
                        HeatingPhase.HEATING -> stringResource(R.string.treatment_phase_heating)
                        HeatingPhase.APPLYING -> stringResource(R.string.treatment_phase_applying)
                        HeatingPhase.DONE -> stringResource(R.string.treatment_phase_done)
                    }

                Triple(
                    "$phaseText • ${temperature ?: "--"}°C",
                    MaterialTheme.colorScheme.tertiary,
                    false,
                )
            }

            is HeatingViewModel.State.Error -> {
                Triple(
                    stringResource(R.string.action_retry_treatment),
                    MaterialTheme.colorScheme.error,
                    enabled,
                )
            }

            else -> {
                Triple(
                    stringResource(R.string.action_start_treatment),
                    MaterialTheme.colorScheme.primary,
                    enabled,
                )
            }
        }

    val pulse by animateFloatAsState(
        targetValue = if (state is HeatingViewModel.State.Heating) 1f else 0.85f,
        label = "pulse",
    )

    Card(
        onClick = {
            if (clickable) onStart()
        },
        modifier =
            Modifier
                .fillMaxWidth()
                .height(72.dp)
                .graphicsLayer {
                    scaleX = pulse
                    scaleY = pulse
                },
        colors =
            elevatedCardColors(
                containerColor = color.copy(alpha = 0.15f),
            ),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Column {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = color,
                )

                if (state is HeatingViewModel.State.Heating) {
                    Text(
                        text = stringResource(R.string.treatment_note_active_session),
                        style = MaterialTheme.typography.bodySmall,
                        color = color.copy(alpha = 0.7f),
                    )
                }
            }

            if (state is HeatingViewModel.State.Heating) {
                PulsingDot(color)
            }
        }
    }
}
