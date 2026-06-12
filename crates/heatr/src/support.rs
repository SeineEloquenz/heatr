//! Database of known bite-healer USB VID/PID pairs and their support status.

/// A USB (VendorId, ProductId) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VidPid {
    pub vid: u16,
    pub pid: u16,
}

impl std::fmt::Display for VidPid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vid, self.pid)
    }
}

/// Describes the support status for a particular device model.
#[derive(Debug, Clone)]
pub struct SupportStatement {
    pub vid: u16,
    pub pid: u16,
    pub vendor_name: &'static str,
    pub product_name: &'static str,
    /// Whether itchcraft can drive this device.
    pub supported: bool,
    /// Optional human-readable comment on the support status.
    pub comment: Option<&'static str>,
}

impl SupportStatement {
    pub fn vid_pid(&self) -> VidPid {
        VidPid {
            vid: self.vid,
            pid: self.pid,
        }
    }
}

const UNTESTED: &str = "heatr hasn't been tested on this model, \
but it is expected to work fine. Feedback welcome – \
please open an issue on the project's issue tracker";

/// All known bite-healer models and their support status.
pub static SUPPORT_STATEMENTS: &[SupportStatement] = &[
    // --- Supported bite healers ---
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0001,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: None,
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0002,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0003,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0004,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0005,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0006,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0xFCA9,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0xFCBA,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: true,
        comment: Some(UNTESTED),
    },
    // --- Unsupported bite healers ---
    SupportStatement {
        vid: 0x10C4,
        pid: 0x8C9B,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it (legacy)",
        supported: false,
        comment: Some("heatr does not work with this legacy model."),
    },
    SupportStatement {
        vid: 0x10C4,
        pid: 0xEA60,
        vendor_name: "mibeTec GmbH",
        product_name: "bite away® pro",
        supported: false,
        comment: Some("Support for this model is on the roadmap for a future release."),
    },
    SupportStatement {
        vid: 0x10C4,
        pid: 0xEAC9,
        vendor_name: "Silicon Laboratories, Inc.",
        product_name: "EFM8UB1",
        supported: false,
        comment: Some("Stock EFM8 chipset; missing bite-healer firmware."),
    },
    SupportStatement {
        vid: 0x32F9,
        pid: 0x0007,
        vendor_name: "Kamedi GmbH",
        product_name: "heat it",
        supported: false,
        comment: Some("heatr is not compatible with this newer model yet. Please open an issue."),
    },
];
