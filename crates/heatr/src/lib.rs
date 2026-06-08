//! Heatr core library.
//!
//! Tech demo for interfacing with heat-based USB insect bite healers.
//!
//! # Example
//!
//! ```no_run
//! use heatr::{Api, Preferences};
//!
//! let api = Api::new();
//! api.info().unwrap();
//!
//! let prefs = Preferences::default();
//! api.start(prefs, |_| {}).unwrap();
//! ```

pub(crate) mod backend;
pub mod device;
pub(crate) mod devices;
pub mod error;
pub(crate) mod heat_it;
pub mod prefs;
pub mod support;

// Re-export the most commonly used types at the crate root.
pub use error::HeatrError;
pub use heat_it::HeatingStatus;
pub use prefs::{Duration, Generation, Preferences, SkinSensitivity};

use device::BiteHealerMetadata;
use devices::find_bite_healers;
use error::Result;
use tracing::{info, warn};

/// The primary API entry point.
pub struct Api;

impl Api {
    /// Creates a new `Api` instance.
    pub fn new() -> Self {
        Api
    }

    /// Shows a list of USB bite healers connected to the host.
    pub fn info(&self) -> Result<Vec<BiteHealerMetadata>> {
        let healers = find_bite_healers()?;
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

    /// Runs the initialization sequence on a connected bite healer.
    ///
    /// Must be called once after connecting the device before the first
    /// `start`. Subsequent `start` calls on the same device session do not
    /// need to re-run init.
    pub fn init(&self) -> Result<()> {
        info!("Searching for bite healer…");

        let candidates = find_bite_healers()?;
        if candidates.is_empty() {
            return Err(HeatrError::NoBiteHealerConnected);
        }

        let candidate = candidates
            .into_iter()
            .find(|c| c.support_statement.supported)
            .ok_or(HeatrError::UnsupportedBiteHealer)?;

        info!(
            "Initializing: {} ({})",
            candidate.product_name(),
            candidate.vendor_name()
        );

        let mut healer = candidate.connect()?;
        healer.self_test()?;
        info!("Bite healer ready.");
        Ok(())
    }

    /// Activates a connected USB bite healer for demonstration purposes.
    ///
    /// Assumes `init` has already been called for this device session.
    /// `on_progress` is called after every status poll during heating.
    pub fn start<F>(&self, preferences: Preferences, on_progress: F) -> Result<()>
    where
        F: FnMut(&HeatingStatus),
    {
        warn!("This app is only a tech demo and NOT for medical use.");
        warn!("The app is NOT SAFE to use for treating insect bites.");
        info!("Searching for bite healer…");

        let candidates = find_bite_healers()?;
        if candidates.is_empty() {
            return Err(HeatrError::NoBiteHealerConnected);
        }

        let candidate = candidates
            .into_iter()
            .find(|c| c.support_statement.supported)
            .ok_or(HeatrError::UnsupportedBiteHealer)?;

        info!(
            "Using bite healer: {} ({})",
            candidate.product_name(),
            candidate.vendor_name()
        );
        info!("Using settings: {}", preferences);

        let mut healer = candidate.connect()?;
        healer.start_with_preferences(&preferences)?;
        healer.monitor(on_progress)?;
        Ok(())
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}
