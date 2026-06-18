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

use futures_timer::Delay;
use futures_util::Stream;
use futures_util::stream;
use tracing::{debug, info};

use crate::backend::BulkTransferDevice;
use crate::error::{HeatrError, Result};
use crate::prefs::Preferences;

/// Phase of a treatment cycle, as reported by GET_STATUS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatingPhase {
    /// Heating element is actively on (`flags == 0x80`).
    Heating,
    /// Element is off but the post-heating apply phase is still in progress
    /// (`flags == 0x00`, `phase != 0x00`).
    Applying,
    /// Treatment cycle fully ended; device is idle (`flags == 0x00`, `phase == 0x00`).
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature {
    raw: u16,
}

impl Temperature {
    pub fn from_raw(raw: u16) -> Temperature {
        Temperature { raw }
    }

    pub fn as_celsius(&self) -> u16 {
        self.raw / 10
    }
}

/// Status snapshot returned by a GET_STATUS + POLL pair.
#[derive(Debug, Clone, Copy)]
pub struct HeatingStatus {
    pub phase: HeatingPhase,
    /// Raw ADC temperature value (Assumed to be degree_celsius = temperature / 10, but not confirmed).
    pub temperature: Temperature,
}

const MIN_RESPONSE_LEN: usize = 2;
/// Maximum number of self-test retries.
const SELF_TEST_RETRIES: u8 = 10;
/// Hardcoded unique-ID region address (not returned by GET_DEVICE_INFO).
const UNIQUE_ID_BASE: u16 = 0xFFC0;
/// Interval between status polls while monitoring a treatment cycle.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

/// A "heat it" device handle.
pub struct HeatItDevice {
    backend: Box<dyn BulkTransferDevice>,
}

impl HeatItDevice {
    /// Creates a new `HeatItDevice` wrapping the given backend.
    pub fn new(backend: Box<dyn BulkTransferDevice>) -> Self {
        Self { backend }
    }

    async fn send(&mut self, request: &[u8], name: &str) -> Result<Vec<u8>> {
        debug!("Sending command: {}", name);
        let response = self.backend.bulk_transfer(request).await?;
        if response.len() < MIN_RESPONSE_LEN {
            return Err(HeatrError::UnexpectedResponseLength {
                got: response.len(),
                expected: MIN_RESPONSE_LEN,
            });
        }
        Ok(response)
    }

    /// Issues `TEST_BOOTLOADER` and returns the raw response.
    pub async fn test_bootloader(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0xB0], "TEST_BOOTLOADER").await
    }

    /// Issues `GET_DEVICE_INFO` and returns the raw response.
    ///
    /// Bytes `[2..=3]` of the response are `base1` (firmware config region
    /// start) and bytes `[4..=5]` are `base2` (serial-number region start).
    pub async fn get_device_info(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x0E, 0x0E], "GET_DEVICE_INFO").await
    }

    /// Issues `READ_MEMORY` for 6 bytes at `addr` and returns the raw response.
    pub async fn read_memory(&mut self, addr: u16) -> Result<Vec<u8>> {
        let hi = (addr >> 8) as u8;
        let lo = (addr & 0xFF) as u8;
        let cksum: u8 = 0x0D_u8.wrapping_add(hi).wrapping_add(lo).wrapping_add(0x06);
        self.send(&[0xFF, 0x0D, hi, lo, 0x06, cksum], "READ_MEMORY")
            .await
    }

    /// Issues `GET_STATUS` and returns the raw response.
    pub async fn get_status(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x02, 0x02], "GET_STATUS").await
    }

    /// Issues `POLL` and returns the raw response.
    ///
    /// Must always be called immediately after `get_status`. The response
    /// mirrors the last GET_STATUS payload with the command echoed in bytes
    /// `[0..=2]`.
    pub async fn poll(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x32, 0x32], "POLL").await
    }

    /// Issues `MSG_START_HEATING` with the given preferences.
    pub async fn msg_start_heating(&mut self, prefs: &Preferences) -> Result<Vec<u8>> {
        let gen_skin: u8 = (prefs.generation.code() << 1) | prefs.skin_sensitivity.code();
        let dur: u8 = prefs.duration.code();
        let checksum: u8 = 0x08_u8.wrapping_add(gen_skin).wrapping_add(dur);
        let request = [0xFF, 0x08, gen_skin, dur, checksum];
        self.send(&request, "MSG_START_HEATING").await
    }

    /// Issues `STOP_HEATING` and returns the raw response.
    pub async fn stop_heating(&mut self) -> Result<Vec<u8>> {
        self.send(&[0xFF, 0x18, 0x18], "STOP_HEATING").await
    }

    /// Polls the device once and returns a status snapshot.
    pub async fn poll_status(&mut self) -> Result<HeatingStatus> {
        let r = self.get_status().await?;
        self.poll().await?;
        let flags = r[4];
        let phase = r[5];
        let phase = if flags == 0x80 {
            HeatingPhase::Heating
        } else if phase != 0x00 {
            HeatingPhase::Applying
        } else {
            HeatingPhase::Done
        };
        Ok(HeatingStatus {
            phase,
            temperature: Temperature {
                raw: ((r[2] as u16) << 8) | (r[3] as u16),
            },
        })
    }

    /// Returns a stream of status snapshots polled from the device.
    ///
    /// The first status is polled immediately; subsequent polls are spaced by
    /// 200ms. The stream ends after yielding the first `Done` status, or
    /// after yielding an error.
    ///
    /// The stream does **not** send `STOP_HEATING`; the caller should invoke
    /// [`stop_heating`](Self::stop_heating) once the stream completes. To
    /// cancel monitoring early, drop the stream and call `stop_heating`.
    pub fn monitor(&mut self) -> impl Stream<Item = Result<HeatingStatus>> + Send + '_ {
        stream::unfold((self, true, false), |(device, first, done)| async move {
            if done {
                return None;
            }
            if !first {
                Delay::new(POLL_INTERVAL).await;
            }
            match device.poll_status().await {
                Ok(status) => {
                    let done = status.phase == HeatingPhase::Done;
                    Some((Ok(status), (device, false, done)))
                }
                Err(e) => Some((Err(e), (device, false, true))),
            }
        })
    }

    /// Runs the full initialization sequence, retrying on USB errors.
    ///
    /// Sequence: TEST_BOOTLOADER → GET_DEVICE_INFO → READ_MEMORY ×8
    /// → GET_STATUS → POLL
    pub async fn self_test(&mut self) -> Result<()> {
        let mut last_err = None;

        'attempt: for attempt in 0..SELF_TEST_RETRIES {
            if attempt > 0 {
                debug!("Self-test retry {}/{}", attempt, SELF_TEST_RETRIES - 1);
                Delay::new(std::time::Duration::from_secs(1)).await;
            }

            macro_rules! try_usb {
                ($call:expr) => {
                    match $call {
                        Err(e @ HeatrError::Usb(_)) | Err(e @ HeatrError::Transfer(_)) => {
                            last_err = Some(e);
                            continue 'attempt;
                        }
                        Err(e) => return Err(e),
                        Ok(r) => r,
                    }
                };
            }

            let r = try_usb!(self.test_bootloader().await);
            debug!("TEST_BOOTLOADER: {:02x?}", r);

            let info = try_usb!(self.get_device_info().await);
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
            let r = try_usb!(self.read_memory(base1).await);
            debug!("READ_MEMORY 0x{:04X}: {:02x?}", base1, r);
            let r = try_usb!(self.read_memory(base1 + 6).await);
            debug!("READ_MEMORY 0x{:04X}: {:02x?}", base1 + 6, r);

            // Read hardcoded unique-ID region (0xFFC0, three 6-byte chunks).
            for offset in [0u16, 6, 12] {
                let addr = UNIQUE_ID_BASE + offset;
                let r = try_usb!(self.read_memory(addr).await);
                debug!("READ_MEMORY 0x{:04X}: {:02x?}", addr, r);
            }

            // Read serial-number region (base2, three 6-byte chunks).
            for offset in [0u16, 6, 12] {
                let addr = base2 + offset;
                let r = try_usb!(self.read_memory(addr).await);
                debug!("READ_MEMORY 0x{:04X}: {:02x?}", addr, r);
            }

            let r = try_usb!(self.get_status().await);
            debug!("GET_STATUS: {:02x?}", r);

            let r = try_usb!(self.poll().await);
            debug!("POLL: {:02x?}", r);

            return Ok(());
        }

        Err(last_err.unwrap_or(HeatrError::Device("Self-test failed".into())))
    }

    /// Sends the start-heating command and logs user guidance.
    pub async fn start_with_preferences(&mut self, prefs: &Preferences) -> Result<()> {
        // Preflight status check required before MSG_START_HEATING.
        let r = self.get_status().await?;
        debug!("GET_STATUS (preflight): {:02x?}", r);
        let r = self.poll().await?;
        debug!("POLL (preflight): {:02x?}", r);

        let r = self.msg_start_heating(prefs).await?;
        debug!("MSG_START_HEATING: {:02x?}", r);

        info!("Device now preheating.");
        info!("Watch the LED closely.");
        info!("It will blink purple, then stop and light up blue.");
        info!("Once the LED turns green, the process has completed.");
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
