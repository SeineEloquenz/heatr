//! Device discovery and connection.

use heatr::HeatItDevice;
use heatr::error::Result;

/// Display info for a detected bite healer.
pub struct DeviceView {
    pub product: String,
    pub vendor: String,
    pub serial: Option<String>,
    pub supported: bool,
}

#[cfg(not(feature = "mock-device"))]
mod imp {
    use super::*;
    use heatr::Api;
    use heatr::device::BiteHealerMetadata;

    impl From<&BiteHealerMetadata> for DeviceView {
        fn from(meta: &BiteHealerMetadata) -> Self {
            Self {
                product: meta.product_name().to_owned(),
                vendor: meta.vendor_name().to_owned(),
                serial: meta.serial_number.clone(),
                supported: meta.supported(),
            }
        }
    }

    /// Scans for connected bite healers, returning the one to display.
    ///
    /// Prefers a supported device, falling back to the first unsupported one
    /// so the user learns why it won't work.
    pub async fn discover() -> Result<Option<DeviceView>> {
        let healers = Api::new().info().await?;
        Ok(healers
            .iter()
            .find(|h| h.supported())
            .or_else(|| healers.first())
            .map(DeviceView::from))
    }

    /// Connects to the first supported bite healer.
    #[expect(dead_code)]
    pub async fn connect() -> Result<HeatItDevice> {
        Api::new().connect().await
    }
}

#[cfg(feature = "mock-device")]
mod imp {
    use super::*;
    use crate::mock::MockBulkTransferDevice;

    /// "Finds" the simulated device.
    pub async fn discover() -> Result<Option<DeviceView>> {
        Ok(Some(DeviceView {
            product: "heat it (simulated)".to_owned(),
            vendor: "heatr mock backend".to_owned(),
            serial: Some("MOCK-0001".to_owned()),
            supported: true,
        }))
    }

    /// Connects to the simulated device.
    #[expect(dead_code)]
    pub async fn connect() -> Result<HeatItDevice> {
        Ok(HeatItDevice::new(Box::new(MockBulkTransferDevice::new())))
    }
}

#[expect(unused_imports)]
pub use imp::{connect, discover};
