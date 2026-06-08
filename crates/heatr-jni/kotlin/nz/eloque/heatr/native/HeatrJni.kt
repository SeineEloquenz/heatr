package nz.eloque.heatr.native

import nz.eloque.heatr.api.Duration
import nz.eloque.heatr.api.Generation
import nz.eloque.heatr.api.SkinSensitivity

internal object HeatrJni {
    init {
        System.loadLibrary("heatr_jni")
    }

    external fun openDevice(fd: Int): Long
    external fun closeDevice(handle: Long)
    external fun runInit(handle: Long)
    external fun getSupportedVidPids(): IntArray

    private external fun startHeating(
        handle: Long,
        duration: Int,
        generation: Int,
        skinSensitivity: Int,
        callback: RawHeatingCallback,
    )

    fun startHeating(
        handle: Long,
        duration: Duration,
        generation: Generation,
        skinSensitivity: SkinSensitivity,
        callback: RawHeatingCallback,
    ) = startHeating(handle, duration.ordinal, generation.ordinal, skinSensitivity.ordinal, callback)

    fun interface RawHeatingCallback {
        fun onProgress(phase: Int, temperature: Int)
    }
}
