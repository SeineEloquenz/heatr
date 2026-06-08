//! Driver for the "heat it" bite healer by Kamedi GmbH.
//!
//! Protocol (all requests are bulk transfers; responses are 12 bytes):
//!
//! | Command           | Request bytes                              |
//! |-------------------|--------------------------------------------|
//! | TEST_BOOTLOADER   | `[0xFF, 0xB0]`                             |
//! | GET_DEVICE_INFO   | `[0xFF, 0x0E, 0x0E]`                       |
//! | READ_MEMORY       | `[0xFF, 0x0D, addr_hi, addr_lo, 0x06, ck]` |
//! | GET_STATUS        | `[0xFF, 0x02, 0x02]`                       |
//! | POLL              | `[0xFF, 0x32, 0x32]`                       |
//! | MSG_START_HEATING | `[0xFF, 0x08, gen_skin, dur, ck]`          |
//! | STOP_HEATING      | `[0xFF, 0x18, 0x18]`                       |
//!
//! `gen_skin = (generation_code << 1) | skin_sensitivity_code`
//!
//! Checksums: `sum(payload_bytes) % 256` where payload_bytes are all bytes
//! after the `0xFF` header and before the checksum itself.
//!
//! See `contrib/frida/PROTOCOL.md` for the full specification, memory layout,
//! and required session sequences derived from Frida tracing.

use tracing::{debug, info, warn};

use crate::backend::BulkTransferDevice;
use crate::error::{HeatrError, Result};
use crate::prefs::Preferences;

/// Status snapshot returned by a GET_STATUS + POLL pair.
pub struct HeatingStatus {
    /// Whether the heating element is currently on.
    pub is_heating: bool,
    /// Whether the treatment cycle has fully ended and the device is idle.
    /// This is the correct exit condition — `!is_heating` alone is not enough,
    /// as the device goes through a post-heating phase before returning to idle.
    pub is_done: bool,
    /// Raw ADC temperature value (~0x46 cold, rises to ~0xE1 at peak).
    pub temperature: u8,
}

const MIN_RESPONSE_LEN: usize = 2;
/// Maximum number of self-test retries.
const SELF_TEST_RETRIES: u8 = 10;
/// Hardcoded unique-ID region address (not returned by GET_DEVICE_INFO).
const UNIQUE_ID_BASE: u16 = 0xFFC0;

/// A "heat it" device handle.
pub struct HeatItDevice {
    backend: Box<dyn BulkTransferDevice>,
}

impl HeatItDevice {
    /// Creates a new `HeatItDevice` wrapping the given backend.
    pub fn new(backend: Box<dyn BulkTransferDevice>) -> Self {
        Self { backend }
    }

    fn send(&mut self, request: &[u8], name: &str) -> Result<Vec<u8>> {
        debug!("Sending command: {}", name);
        let response = self.backend.bulk_transfer(request)?;
        if response.len() < MIN_RESPONSE_LEN {
            return Err(HeatrError::UnexpectedResponseLength {
                got: response.len(),
                expected: MIN_RESPONSE_LEN,
            });
        }
        Ok(response)
    }

    /// Issues `TEST_BOOTLOADER` and returns the raw response.
    pub fn test_bootloader(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0xB0], "TEST_BOOTLOADER")
    }

    /// Issues `GET_DEVICE_INFO` and returns the raw response.
    ///
    /// Bytes `[2..=3]` of the response are `base1` (firmware config region
    /// start) and bytes `[4..=5]` are `base2` (serial-number region start).
    pub fn get_device_info(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x0E, 0x0E], "GET_DEVICE_INFO")
    }

    /// Issues `READ_MEMORY` for 6 bytes at `addr` and returns the raw response.
    pub fn read_memory(&mut self, addr: u16) -> Result<Vec<u8>> {
        let hi = (addr >> 8) as u8;
        let lo = (addr & 0xFF) as u8;
        let cksum: u8 = 0x0D_u8
            .wrapping_add(hi)
            .wrapping_add(lo)
            .wrapping_add(0x06);
        self.send(&[0xFF, 0x0D, hi, lo, 0x06, cksum], "READ_MEMORY")
    }

    /// Issues `GET_STATUS` and returns the raw response.
    pub fn get_status(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x02, 0x02], "GET_STATUS")
    }

    /// Issues `POLL` and returns the raw response.
    ///
    /// Must always be called immediately after `get_status`. The response
    /// mirrors the last GET_STATUS payload with the command echoed in bytes
    /// `[0..=2]`.
    pub fn poll(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x32, 0x32], "POLL")
    }

    /// Issues `MSG_START_HEATING` with the given preferences.
    pub fn msg_start_heating(&mut self, prefs: &Preferences) -> Result<Vec<u8>> {
        let gen_skin: u8 = (prefs.generation.code() << 1) | prefs.skin_sensitivity.code();
        let dur: u8 = prefs.duration.code();
        let checksum: u8 = 0x08_u8.wrapping_add(gen_skin).wrapping_add(dur);
        let request = [0xFF, 0x08, gen_skin, dur, checksum];
        self.send(&request, "MSG_START_HEATING")
    }

    /// Issues `STOP_HEATING` and returns the raw response.
    pub fn stop_heating(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x18, 0x18], "STOP_HEATING")
    }

    /// Polls the device once and returns a status snapshot.
    pub fn poll_status(&mut self) -> Result<HeatingStatus> {
        let r = self.get_status()?;
        self.poll()?;
        let flags = r[4];
        let phase = r[5];
        Ok(HeatingStatus {
            is_heating: flags == 0x80,
            is_done: flags == 0x00 && phase == 0x00,
            temperature: r[3],
        })
    }

    /// Polls until the treatment cycle fully completes, calling `on_progress`
    /// after each poll.
    ///
    /// Sends `STOP_HEATING` once the device returns to idle.
    pub fn monitor<F>(&mut self, mut on_progress: F) -> Result<()>
    where
        F: FnMut(&HeatingStatus),
    {
        loop {
            let status = self.poll_status()?;
            let done = status.is_done;
            on_progress(&status);
            if done {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        self.stop_heating()?;
        Ok(())
    }

    /// Runs the full initialization sequence, retrying on USB errors.
    ///
    /// Sequence: TEST_BOOTLOADER → GET_DEVICE_INFO → READ_MEMORY ×8
    /// → GET_STATUS → POLL
    pub fn self_test(&mut self) -> Result<()> {
        let mut last_err = None;

        'attempt: for attempt in 0..SELF_TEST_RETRIES {
            if attempt > 0 {
                debug!("Self-test retry {}/{}", attempt, SELF_TEST_RETRIES - 1);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            macro_rules! try_usb {
                ($call:expr) => {
                    match $call {
                        Err(e @ HeatrError::Usb(_))
                        | Err(e @ HeatrError::Transfer(_)) => {
                            last_err = Some(e);
                            continue 'attempt;
                        }
                        Err(e) => return Err(e),
                        Ok(r) => r,
                    }
                };
            }

            let r = try_usb!(self.test_bootloader());
            debug!("TEST_BOOTLOADER: {:02x?}", r);

            let info = try_usb!(self.get_device_info());
            debug!("GET_DEVICE_INFO: {:02x?}", info);

            // Extract the two base addresses returned by GET_DEVICE_INFO.
            if info.len() < 6 {
                return Err(HeatrError::Device(
                    "GET_DEVICE_INFO response too short".into(),
                ));
            }
            let base1 = (info[2] as u16) << 8 | info[3] as u16;
            let base2 = (info[4] as u16) << 8 | info[5] as u16;

            // Read firmware config region (base1, two 6-byte chunks).
            let r = try_usb!(self.read_memory(base1));
            debug!("READ_MEMORY 0x{:04X}: {:02x?}", base1, r);
            let r = try_usb!(self.read_memory(base1 + 6));
            debug!("READ_MEMORY 0x{:04X}: {:02x?}", base1 + 6, r);

            // Read hardcoded unique-ID region (0xFFC0, three 6-byte chunks).
            for offset in [0u16, 6, 12] {
                let addr = UNIQUE_ID_BASE + offset;
                let r = try_usb!(self.read_memory(addr));
                debug!("READ_MEMORY 0x{:04X}: {:02x?}", addr, r);
            }

            // Read serial-number region (base2, three 6-byte chunks).
            for offset in [0u16, 6, 12] {
                let addr = base2 + offset;
                let r = try_usb!(self.read_memory(addr));
                debug!("READ_MEMORY 0x{:04X}: {:02x?}", addr, r);
            }

            let r = try_usb!(self.get_status());
            debug!("GET_STATUS: {:02x?}", r);

            let r = try_usb!(self.poll());
            debug!("POLL: {:02x?}", r);

            return Ok(());
        }

        Err(last_err.unwrap_or(HeatrError::Device("Self-test failed".into())))
    }

    /// Sends the start-heating command and logs user guidance.
    pub fn start_with_preferences(&mut self, prefs: &Preferences) -> Result<()> {
        // Preflight status check required before MSG_START_HEATING.
        let r = self.get_status()?;
        debug!("GET_STATUS (preflight): {:02x?}", r);
        let r = self.poll()?;
        debug!("POLL (preflight): {:02x?}", r);

        let r = self.msg_start_heating(prefs)?;
        debug!("MSG_START_HEATING: {:02x?}", r);

        info!("Device now preheating.");
        info!("Watch the LED closely.");
        info!("It will blink purple, then stop and light up blue.");
        warn!("While using this app, your bite healer is NOT SAFE for use on human skin.");
        info!("Once the LED turns green, the tech demo has completed.");
        Ok(())
    }

    /// Returns the product name reported by the USB device.
    pub fn product_name(&self) -> Option<String> {
        self.backend.product_name()
    }

    /// Returns the serial number reported by the USB device.
    pub fn serial_number(&self) -> Option<String> {
        self.backend.serial_number()
    }
}
