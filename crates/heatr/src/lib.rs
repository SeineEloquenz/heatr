//! Heatr core library.
//!
//! Tech demo for interfacing with heat-based USB insect bite healers.
//!
//! # Example
//!
//! All device I/O is async and runtime-agnostic: the futures run on any
//! executor (glib, smol, tokio, or a simple `block_on` such as `pollster`).
//!
//! ```no_run
//! # #[cfg(not(target_os = "android"))]
//! # pollster::block_on(async {
//! use heatr::{Api, Preferences};
//!
//! let api = Api::new();
//! api.info().await.unwrap();
//!
//! let prefs = Preferences::default();
//! api.start(prefs, |_| {}).await.unwrap();
//! # });
//! ```

pub mod backend;
pub mod device;
#[cfg(not(target_os = "android"))]
pub(crate) mod devices;
pub mod error;
pub mod heat_it;
pub mod prefs;
pub mod support;

// Re-export the most commonly used types at the crate root.
pub use backend::{BulkTransferDevice, UsbBulkTransferDevice};
pub use error::HeatrError;
pub use heat_it::{HeatItDevice, HeatingPhase, HeatingStatus};
pub use prefs::{Duration, Generation, Preferences, SkinSensitivity};

// The Api struct and device-discovery logic rely on nusb::list_devices, which
// is not available on Android (where the OS enumerates USB devices instead).
#[cfg(not(target_os = "android"))]
use device::BiteHealerMetadata;
#[cfg(not(target_os = "android"))]
use devices::find_bite_healers;
#[cfg(not(target_os = "android"))]
use error::Result;
#[cfg(not(target_os = "android"))]
use futures_util::StreamExt;
#[cfg(not(target_os = "android"))]
use tracing::{info, warn};

/// The primary API entry point.
#[cfg(not(target_os = "android"))]
pub struct Api;

#[cfg(not(target_os = "android"))]
impl Api {
    /// Creates a new `Api` instance.
    pub fn new() -> Self {
        Api
    }

    /// Shows a list of USB bite healers connected to the host.
    pub async fn info(&self) -> Result<Vec<BiteHealerMetadata>> {
        let healers = find_bite_healers().await?;
        if healers.is_empty() {
            info!("No known bite healers detected");
        } else {
            info!(
                "Detected {} bite healer{}",
                healers.len(),
                if healers.len() == 1 { "" } else { "s" }
            );
        }
        Ok(healers)
    }

    /// Connects to the first supported bite healer and returns the device
    /// session.
    ///
    /// The returned [`HeatItDevice`] can be used directly for `self_test`
    /// followed by one or more start/monitor/stop cycles, without
    /// re-discovering the device in between.
    pub async fn connect(&self) -> Result<HeatItDevice> {
        info!("Searching for bite healer…");

        let candidates = find_bite_healers().await?;
        if candidates.is_empty() {
            return Err(HeatrError::NoBiteHealerConnected);
        }

        let candidate = candidates
            .into_iter()
            .find(|c| c.support_statement.supported)
            .ok_or(HeatrError::UnsupportedBiteHealer)?;

        info!(
            "Connecting to bite healer: {} ({})",
            candidate.product_name(),
            candidate.vendor_name()
        );

        candidate.connect().await
    }

    /// Runs the initialization sequence on a connected bite healer.
    ///
    /// Must be called once after connecting the device before the first
    /// `start`. Subsequent `start` calls on the same device session do not
    /// need to re-run init.
    pub async fn init(&self) -> Result<()> {
        let mut healer = self.connect().await?;
        healer.self_test().await?;
        info!("Bite healer ready.");
        Ok(())
    }

    /// Activates a connected USB bite healer for demonstration purposes.
    ///
    /// Assumes `init` has already been called for this device session.
    /// `on_progress` is called after every status poll during heating.
    pub async fn start<F>(&self, preferences: Preferences, mut on_progress: F) -> Result<()>
    where
        F: FnMut(&HeatingStatus),
    {
        warn!("This app is NOT a certified medical product.");

        let mut healer = self.connect().await?;
        info!("Using settings: {}", preferences);

        healer.start_with_preferences(&preferences).await?;
        {
            let stream = healer.monitor();
            futures_util::pin_mut!(stream);
            while let Some(status) = stream.next().await {
                on_progress(&status?);
            }
        }
        healer.stop_heating().await?;
        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}
