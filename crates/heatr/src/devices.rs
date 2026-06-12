//! Device discovery: enumerate USB devices and match against known VID/PIDs.

use nusb::list_devices;
use tracing::debug;

use crate::device::BiteHealerMetadata;
use crate::error::Result;
use crate::support::{SUPPORT_STATEMENTS, VidPid};

/// Finds all bite healers (supported and unsupported) that are currently
/// connected to this host.
pub async fn find_bite_healers() -> Result<Vec<BiteHealerMetadata>> {
    let mut results = Vec::new();

    for device in list_devices().await? {
        let vid_pid = VidPid {
            vid: device.vendor_id(),
            pid: device.product_id(),
        };

        let Some(statement) = SUPPORT_STATEMENTS.iter().find(|s| s.vid_pid() == vid_pid) else {
            debug!("Ignoring USB device {}", vid_pid);
            continue;
        };

        debug!("Detected bite healer {}", vid_pid);

        results.push(BiteHealerMetadata {
            usb_product_name: device.product_string().map(ToOwned::to_owned),
            serial_number: device.serial_number().map(ToOwned::to_owned),
            support_statement: statement,
            usb_device: device,
        });
    }

    Ok(results)
}
