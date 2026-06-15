//! USB bulk-transfer backend abstraction.

use async_trait::async_trait;
use futures_timer::Delay;
use futures_util::future::{Either, select};
use futures_util::pin_mut;
use nusb::{
    descriptors::TransferType,
    transfer::{Buffer, BulkOrInterrupt, Completion, Direction, EndpointDirection},
};
use tracing::debug;

use crate::error::{HeatrError, Result};

const REPLY_BUFFER_SIZE: usize = 64;
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Trait for a USB device capable of bulk transfers.
///
/// This abstraction exists so that the higher-level `HeatItDevice` can be
/// tested without a real USB device (by providing a mock implementation).
#[async_trait]
pub trait BulkTransferDevice: Send {
    /// Sends `request` via USB bulk transfer and returns the device response.
    async fn bulk_transfer(&mut self, request: &[u8]) -> Result<Vec<u8>>;

    /// Human-readable product name reported by the USB device.
    fn product_name(&self) -> Option<String>;

    /// Serial number reported by the USB device.
    fn serial_number(&self) -> Option<String>;
}

/// Submits `buf` on `ep` and waits for the transfer to complete, cancelling
/// it if it does not complete within `timeout`.
///
/// Mirrors `Endpoint::transfer_blocking`: on timeout the transfer is
/// cancelled and the returned `Completion` has a status of
/// `TransferError::Cancelled`.
async fn transfer_with_timeout<EpType, Dir>(
    ep: &mut nusb::Endpoint<EpType, Dir>,
    buf: Buffer,
    timeout: std::time::Duration,
) -> Completion
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    ep.submit(buf);
    {
        let completion = ep.next_complete();
        pin_mut!(completion);
        if let Either::Left((completion, _)) = select(completion, Delay::new(timeout)).await {
            return completion;
        }
    }
    // Timed out: request cancellation. The cancelled transfer is still
    // returned through next_complete.
    ep.cancel_all();
    ep.next_complete().await
}

/// A real USB bulk-transfer device backed by nusb.
pub struct UsbBulkTransferDevice {
    ep_out: nusb::Endpoint<nusb::transfer::Bulk, nusb::transfer::Out>,
    ep_in: nusb::Endpoint<nusb::transfer::Bulk, nusb::transfer::In>,
    product_name: Option<String>,
    serial_number: Option<String>,
}

impl UsbBulkTransferDevice {
    /// Opens the given USB device, selects the non-iAP configuration, and
    /// locates its bulk endpoints.
    pub async fn open(device_info: &nusb::DeviceInfo) -> Result<Self> {
        let product_name = device_info.product_string().map(ToOwned::to_owned);
        let serial_number = device_info.serial_number().map(ToOwned::to_owned);
        let device = device_info.open().await?;

        // The heat it device exposes two configurations:
        //   1 = iAP (Apple Accessory Protocol, subclass 0xF0) — selected by
        //       default on Linux; returns iAP identification packets instead
        //       of heat-it responses.
        //   2 = standard USB — used by the Android app; responds normally.
        // Dynamically find the first non-iAP config so we don't hardcode the
        // config number.
        const IAP_SUBCLASS: u8 = 0xF0;
        let config_value = device
            .configurations()
            .find(|cfg| {
                cfg.interfaces().all(|iface| {
                    iface
                        .alt_settings()
                        .all(|alt| alt.subclass() != IAP_SUBCLASS)
                })
            })
            .map(|cfg| cfg.configuration_value())
            .ok_or_else(|| {
                HeatrError::EndpointNotFound("No non-iAP USB configuration found".into())
            })?;

        // SET_CONFIGURATION resets all endpoint toggle bits, clears stall/halt
        // state, and switches to the standard USB config.
        device.set_configuration(config_value).await?;

        // Give the device firmware time to reinitialize its USB stack after
        // the configuration switch before we start sending commands.
        Delay::new(std::time::Duration::from_millis(100)).await;

        Self::setup_endpoints(device, product_name, serial_number).await
    }

    /// Opens a device from a pre-opened file descriptor.
    ///
    /// Intended for Android, where `UsbManager` opens the device and provides
    /// a file descriptor via `UsbDeviceConnection.getFileDescriptor()`. The OS
    /// has already selected the correct (non-iAP) configuration, so this path
    /// skips `SET_CONFIGURATION` and uses whichever configuration is active.
    #[cfg(any(target_os = "android", target_os = "linux"))]
    pub async fn from_fd(fd: std::os::fd::OwnedFd) -> Result<Self> {
        let device = nusb::Device::from_fd(fd).await?;
        Self::setup_endpoints(device, None, None).await
    }

    async fn find_configuration(device: &nusb::Device) -> Result<u8> {
        // Select the non-iAP configuration.
        let mut selected_config = None;

        for config in device.configurations() {
            let is_iap = config.interfaces().any(|interface| {
                interface
                    .alt_settings()
                    .any(|alt| alt.class() == 0xff && alt.subclass() == 0xf0)
            });

            if !is_iap {
                selected_config = Some(config.configuration_value());
                break;
            }
        }

        let selected_config = selected_config.ok_or_else(|| {
            HeatrError::EndpointNotFound("No non-iAP USB configuration found".into())
        })?;

        Ok(selected_config)
    }

    /// Discovers bulk IN/OUT endpoints on the USB configuration (not the iAP
    /// configuration) and claims the interface.
    async fn setup_endpoints(
        device: nusb::Device,
        product_name: Option<String>,
        serial_number: Option<String>,
    ) -> Result<Self> {
        device
            .set_configuration(UsbBulkTransferDevice::find_configuration(&device).await?)
            .await?;

        let config = device.active_configuration()?;

        let mut endpoint_out = None;
        let mut endpoint_in = None;
        let mut interface_number = None;

        'outer: for interface in config.interfaces() {
            for alt in interface.alt_settings() {
                // Skip iAP interfaces just in case.
                if alt.class() == 0xff && alt.subclass() == 0xf0 {
                    continue;
                }

                let mut out = None;
                let mut input = None;

                for ep in alt.endpoints() {
                    if ep.transfer_type() != TransferType::Bulk {
                        continue;
                    }

                    match ep.direction() {
                        Direction::Out => out = Some(ep.address()),
                        Direction::In => input = Some(ep.address()),
                    }
                }

                if let (Some(out_ep), Some(in_ep)) = (out, input) {
                    endpoint_out = Some(out_ep);
                    endpoint_in = Some(in_ep);
                    interface_number = Some(interface.interface_number());
                    break 'outer;
                }
            }
        }

        let endpoint_out = endpoint_out
            .ok_or_else(|| HeatrError::EndpointNotFound("No bulk OUT endpoint found".into()))?;

        let endpoint_in = endpoint_in
            .ok_or_else(|| HeatrError::EndpointNotFound("No bulk IN endpoint found".into()))?;

        let interface_number = interface_number.ok_or_else(|| {
            HeatrError::EndpointNotFound("No interface with bulk IN/OUT endpoints found".into())
        })?;

        let interface = device.detach_and_claim_interface(interface_number).await?;

        let ep_out =
            interface.endpoint::<nusb::transfer::Bulk, nusb::transfer::Out>(endpoint_out)?;

        let ep_in = interface.endpoint::<nusb::transfer::Bulk, nusb::transfer::In>(endpoint_in)?;

        Ok(Self {
            ep_out,
            ep_in,
            product_name,
            serial_number,
        })
    }
}

#[async_trait]
impl BulkTransferDevice for UsbBulkTransferDevice {
    async fn bulk_transfer(&mut self, request: &[u8]) -> Result<Vec<u8>> {
        transfer_with_timeout(&mut self.ep_out, request.to_vec().into(), TIMEOUT)
            .await
            .status?;

        let response =
            transfer_with_timeout(&mut self.ep_in, Buffer::new(REPLY_BUFFER_SIZE), TIMEOUT).await;
        response.status?;

        let buf = response.buffer.into_vec();

        debug!("Response ({} bytes): {:02x?}", buf.len(), &buf);

        Ok(buf)
    }

    fn product_name(&self) -> Option<String> {
        self.product_name.clone()
    }

    fn serial_number(&self) -> Option<String> {
        self.serial_number.clone()
    }
}
