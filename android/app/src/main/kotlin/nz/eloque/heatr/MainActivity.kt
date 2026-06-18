package nz.eloque.heatr

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Surface
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import nz.eloque.heatr.api.Heatr
import nz.eloque.heatr.ui.HeatrScaffold
import nz.eloque.heatr.ui.heater.HeatingViewModel
import nz.eloque.heatr.ui.heater.HeatrScreen
import nz.eloque.heatr.ui.theme.HeatrTheme

class MainActivity : ComponentActivity() {
    private companion object {
        const val ACTION_USB_PERMISSION = "nz.eloque.heatr.USB_PERMISSION"
    }

    private lateinit var usbManager: UsbManager
    private val viewModel: HeatingViewModel by viewModels()

    private var statusText by mutableStateOf("No device connected")
    private var hasDevice by mutableStateOf(false)
    private var currentDevice: UsbDevice? = null

    private val usbReceiver =
        object : BroadcastReceiver() {
            override fun onReceive(
                context: Context,
                intent: Intent,
            ) {
                when (intent.action) {
                    ACTION_USB_PERMISSION -> {
                        @Suppress("DEPRECATION")
                        val device: UsbDevice? = intent.getParcelableExtra(UsbManager.EXTRA_DEVICE)
                        if (intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false)) {
                            device?.let { openDevice(it) }
                        } else {
                            statusText = "USB permission denied"
                        }
                    }

                    UsbManager.ACTION_USB_DEVICE_ATTACHED -> {
                        @Suppress("DEPRECATION")
                        val device: UsbDevice? = intent.getParcelableExtra(UsbManager.EXTRA_DEVICE)
                        device?.let { handleDeviceAttached(it) }
                    }

                    UsbManager.ACTION_USB_DEVICE_DETACHED -> {
                        statusText = "Device disconnected"
                        currentDevice = null
                        hasDevice = false
                        viewModel.deviceDisconnected()
                    }
                }
            }
        }

    @OptIn(ExperimentalMaterial3Api::class)
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        usbManager = getSystemService(USB_SERVICE) as UsbManager

        val filter =
            IntentFilter().apply {
                addAction(ACTION_USB_PERMISSION)
                addAction(UsbManager.ACTION_USB_DEVICE_ATTACHED)
                addAction(UsbManager.ACTION_USB_DEVICE_DETACHED)
            }
        ContextCompat.registerReceiver(this, usbReceiver, filter, ContextCompat.RECEIVER_NOT_EXPORTED)

        setContent {
            val state by viewModel.state.collectAsStateWithLifecycle()

            LaunchedEffect(state) {
                when (state) {
                    is HeatingViewModel.State.NoDevice -> {
                        statusText = "No device connected"
                    }

                    is HeatingViewModel.State.DeviceReady -> {
                        statusText = "Device ready"
                    }

                    is HeatingViewModel.State.Error -> {
                        statusText = "Error: ${(state as HeatingViewModel.State.Error).message}"
                    }

                    else -> {}
                }
            }

            HeatrTheme {
                HeatrScaffold {
                    HeatrScreen(
                        statusText = statusText,
                        state = state,
                        hasDevice = hasDevice,
                        onInit = { viewModel.runInit() },
                        onStart = { duration, generation, skin ->
                            viewModel.startHeating(duration, generation, skin)
                        },
                    )
                }
            }
        }

        @Suppress("DEPRECATION")
        val attachedDevice = intent?.getParcelableExtra<UsbDevice>(UsbManager.EXTRA_DEVICE)
        attachedDevice?.let { handleDeviceAttached(it) } ?: checkExistingDevices()
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterReceiver(usbReceiver)
    }

    private fun checkExistingDevices() {
        usbManager.deviceList.values
            .firstOrNull { isSupported(it) }
            ?.let { handleDeviceAttached(it) }
    }

    private fun handleDeviceAttached(device: UsbDevice) {
        if (!isSupported(device)) return
        currentDevice = device
        hasDevice = true
        statusText = "Device found: ${device.productName ?: "heat it"}"

        if (usbManager.hasPermission(device)) {
            openDevice(device)
        } else {
            val permissionIntent =
                PendingIntent.getBroadcast(
                    this,
                    0,
                    Intent(ACTION_USB_PERMISSION),
                    PendingIntent.FLAG_IMMUTABLE,
                )
            usbManager.requestPermission(device, permissionIntent)
        }
    }

    private fun openDevice(device: UsbDevice) {
        val connection =
            usbManager.openDevice(device) ?: run {
                statusText = "Failed to open USB connection"
                return
            }
        viewModel.openDevice(connection.fileDescriptor)
    }

    private fun isSupported(device: UsbDevice): Boolean =
        Heatr.supportedUsbIds().any { it.vid == device.vendorId && it.pid == device.productId }
}
