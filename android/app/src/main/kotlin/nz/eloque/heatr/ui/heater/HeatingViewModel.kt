package nz.eloque.heatr.ui.heater

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import nz.eloque.heatr.api.Duration
import nz.eloque.heatr.api.Generation
import nz.eloque.heatr.api.HeatingPhase
import nz.eloque.heatr.api.Heatr
import nz.eloque.heatr.api.HeatrDevice
import nz.eloque.heatr.api.SkinSensitivity

class HeatingViewModel : ViewModel() {
    sealed class State {
        object NoDevice : State()

        object DeviceReady : State()

        data class Heating(
            val phase: HeatingPhase,
            val temperature: Int,
        ) : State()

        object Done : State()

        data class Error(
            val message: String,
        ) : State()
    }

    private val _state = MutableStateFlow<State>(State.NoDevice)
    val state: StateFlow<State> = _state.asStateFlow()

    private var device: HeatrDevice? = null

    fun openDevice(fd: Int) {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                device = Heatr.openDevice(fd)
                _state.value = State.DeviceReady
                device?.runInit()
            } catch (e: RuntimeException) {
                _state.value = State.Error(e.message ?: "Failed to open device")
            }
        }
    }

    fun runInit() {
        val dev = device ?: return
        viewModelScope.launch(Dispatchers.IO) {
            try {
                dev.runInit()
            } catch (e: RuntimeException) {
                _state.value = State.Error(e.message ?: "Init failed")
            }
        }
    }

    fun startHeating(
        duration: Duration,
        generation: Generation,
        skinSensitivity: SkinSensitivity,
    ) {
        val dev = device ?: return
        viewModelScope.launch(Dispatchers.IO) {
            try {
                dev.startHeating(duration, generation, skinSensitivity) { status ->
                    _state.value = State.Heating(status.phase, status.temperature)
                }
                _state.value = State.Done
            } catch (e: RuntimeException) {
                _state.value = State.Error(e.message ?: "Heating failed")
            }
        }
    }

    fun deviceDisconnected() {
        releaseDevice()
        _state.value = State.NoDevice
    }

    private fun releaseDevice() {
        device?.close()
        device = null
    }

    override fun onCleared() {
        super.onCleared()
        releaseDevice()
    }
}
