//! Device metadata types.

use crate::backend::UsbBulkTransferDevice;
use crate::error::Result;
use crate::heat_it::HeatItDevice;
use crate::support::SupportStatement;

/// Metadata about a detected bite healer.
#[derive(Debug)]
pub struct BiteHealerMetadata {
    /// USB product name string (best-effort, may be None if not readable).
    pub usb_product_name: Option<String>,
    /// USB serial number string (best-effort).
    pub serial_number: Option<String>,
    /// The heatr support record for this device.
    pub support_statement: &'static SupportStatement,
    /// The rusb device handle for opening a connection.
    pub(crate) usb_device: nusb::DeviceInfo,
}

impl BiteHealerMetadata {
    /// Canonical product name from heatr's perspective.
    pub fn product_name(&self) -> &str {
        self.support_statement.product_name
    }

    /// Canonical vendor name from heatr's perspective.
    pub fn vendor_name(&self) -> &str {
        self.support_statement.vendor_name
    }

    /// Whether heatr can drive this device.
    pub fn supported(&self) -> bool {
        self.support_statement.supported
    }

    /// Opens a connection to this device and returns a ready `HeatItDevice`.
    pub async fn connect(&self) -> Result<HeatItDevice> {
        let backend = UsbBulkTransferDevice::open(&self.usb_device).await?;
        Ok(HeatItDevice::new(Box::new(backend)))
    }
}
