package nz.eloque.heatr.api

fun interface HeatingCallback {
    fun onProgress(status: HeatingStatus)
}
