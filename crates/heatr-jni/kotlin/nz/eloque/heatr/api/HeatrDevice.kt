package nz.eloque.heatr.api

import nz.eloque.heatr.native.HeatrJni

class HeatrDevice internal constructor(private val handle: Long) : AutoCloseable {

    fun runInit() = HeatrJni.runInit(handle)

    fun startHeating(
        duration: Duration,
        generation: Generation,
        skinSensitivity: SkinSensitivity,
        onProgress: HeatingCallback,
    ) = HeatrJni.startHeating(handle, duration, generation, skinSensitivity) { phase, temperature ->
        onProgress.onProgress(HeatingStatus(HeatingPhase.entries[phase], temperature))
    }

    override fun close() = HeatrJni.closeDevice(handle)
}
