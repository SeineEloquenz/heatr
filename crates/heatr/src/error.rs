//! Error types for heatr.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, HeatrError>;

#[derive(Debug, Error)]
pub enum HeatrError {
    #[error("USB error: {0}")]
    Usb(#[from] nusb::Error),

    #[error("Transfer error: {0}")]
    Transfer(#[from] nusb::transfer::TransferError),

    #[error("Configuration error: {0}")]
    Configuration(#[from] nusb::ActiveConfigurationError),

    #[error("No bite healer connected")]
    NoBiteHealerConnected,

    #[error("The connected bite healer is not supported by this version of heatr")]
    UnsupportedBiteHealer,

    #[error("Backend initialization error: {0}")]
    BackendInit(String),

    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),

    #[error("Unexpected response length: got {got}, expected {expected}")]
    UnexpectedResponseLength { got: usize, expected: usize },

    #[error("Device error: {0}")]
    Device(String),

    #[error("Invalid preference value '{value}' for {field}. Valid values: {valid}")]
    InvalidPreference {
        field: String,
        value: String,
        valid: String,
    },
}
