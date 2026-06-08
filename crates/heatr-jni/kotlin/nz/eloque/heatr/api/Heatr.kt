package nz.eloque.heatr.api

import nz.eloque.heatr.native.HeatrJni

object Heatr {
    fun openDevice(fd: Int): HeatrDevice = HeatrDevice(HeatrJni.openDevice(fd))

    fun supportedUsbIds(): List<UsbId> {
        val raw = HeatrJni.getSupportedVidPids()
        return (raw.indices step 2).map { i -> UsbId(raw[i], raw[i + 1]) }
    }
}
