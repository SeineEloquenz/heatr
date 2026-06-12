//! In-process simulated bite healer for hardware-free UI testing.
use std::time::Instant;

use async_trait::async_trait;
use futures_timer::Delay;
use heatr::BulkTransferDevice;
use heatr::error::Result;

/// Wire command bytes (the byte following the `0xFF` header).
const CMD_GET_DEVICE_INFO: u8 = 0x0E;
const CMD_GET_STATUS: u8 = 0x02;
const CMD_START_HEATING: u8 = 0x08;
const CMD_STOP_HEATING: u8 = 0x18;

/// Raw ADC temperature endpoints (cold → peak).
const TEMP_COLD: f32 = 0x46 as f32;
const TEMP_PEAK: f32 = 0xE1 as f32;

/// How long the simulated cycle spends in each phase, in seconds.
const HEATING_SECS: f32 = 4.0;
const APPLYING_SECS: f32 = 2.0;

/// A fake bite healer that simulates the heat-it protocol.
pub struct MockBulkTransferDevice {
    /// When the current heating cycle began, or `None` while idle.
    started_at: Option<Instant>,
}

impl MockBulkTransferDevice {
    pub fn new() -> Self {
        Self { started_at: None }
    }

    /// Builds a 12-byte GET_STATUS response reflecting elapsed cycle time.
    fn status_response(&self) -> Vec<u8> {
        let mut r = vec![0u8; 12];
        let Some(started_at) = self.started_at else {
            // Idle: report done/cold (matches the preflight and self-test polls).
            r[3] = TEMP_COLD as u8;
            return r;
        };

        let elapsed = started_at.elapsed().as_secs_f32();
        let (flags, phase, temp) = if elapsed < HEATING_SECS {
            // Heating: temperature ramps cold → peak.
            let t = elapsed / HEATING_SECS;
            (0x80, 0x01, TEMP_COLD + (TEMP_PEAK - TEMP_COLD) * t)
        } else if elapsed < HEATING_SECS + APPLYING_SECS {
            // Applying: element off, temperature drifts back down.
            let t = (elapsed - HEATING_SECS) / APPLYING_SECS;
            (0x00, 0x01, TEMP_PEAK - (TEMP_PEAK - TEMP_COLD) * 0.5 * t)
        } else {
            // Done: fully idle.
            (0x00, 0x00, TEMP_COLD)
        };

        r[3] = temp as u8;
        r[4] = flags;
        r[5] = phase;
        r
    }
}

#[async_trait]
impl BulkTransferDevice for MockBulkTransferDevice {
    async fn bulk_transfer(&mut self, request: &[u8]) -> Result<Vec<u8>> {
        // A touch of latency so transitions feel like a real bus rather than
        // resolving instantly.
        Delay::new(std::time::Duration::from_millis(5)).await;

        let command = request.get(1).copied().unwrap_or(0);
        let response = match command {
            CMD_START_HEATING => {
                self.started_at = Some(Instant::now());
                vec![0u8; 12]
            }
            CMD_STOP_HEATING => {
                self.started_at = None;
                vec![0u8; 12]
            }
            CMD_GET_STATUS => self.status_response(),
            CMD_GET_DEVICE_INFO => {
                // base1 = 0x1000 (firmware config), base2 = 0x2000 (serial).
                let mut r = vec![0u8; 12];
                r[2] = 0x10;
                r[3] = 0x00;
                r[4] = 0x20;
                r[5] = 0x00;
                r
            }
            // TEST_BOOTLOADER, READ_MEMORY, POLL and anything else: a generic
            // well-formed response is all the driver needs.
            _ => vec![0u8; 12],
        };
        Ok(response)
    }

    fn product_name(&self) -> Option<String> {
        Some("heat it (simulated)".to_owned())
    }

    fn serial_number(&self) -> Option<String> {
        Some("MOCK-0001".to_owned())
    }
}
