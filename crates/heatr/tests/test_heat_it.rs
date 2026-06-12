//! Tests for the heat-it protocol, mirroring the original Python test suite.
//!
//! These tests use only the public API and derive expected bytes from the
//! known protocol spec, so no real device is needed.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::{StreamExt, pin_mut};
use heatr::backend::BulkTransferDevice;
use heatr::heat_it::{HeatItDevice, HeatingPhase};
use heatr::prefs::{Duration, Generation, Preferences, SkinSensitivity};

// ---------------------------------------------------------------------------
// Helper: compute the expected MSG_START_HEATING request bytes from a
// Preferences using the same formula as the protocol (public knowledge).
//
// gen_skin = (generation_code << 1) | skin_sensitivity_code
//   child=0, adult=1
//   sensitive=0, regular=1
// cksum = 0x08 + gen_skin + dur   (wrapping byte)
// ---------------------------------------------------------------------------
fn expected_request(prefs: &Preferences) -> Vec<u8> {
    let gen_code: u8 = match prefs.generation {
        Generation::Child => 0,
        Generation::Adult => 1,
    };
    let skin_code: u8 = match prefs.skin_sensitivity {
        SkinSensitivity::Sensitive => 0,
        SkinSensitivity::Regular => 1,
    };
    let dur_code: u8 = match prefs.duration {
        Duration::Short => 0,
        Duration::Medium => 1,
        Duration::Long => 2,
    };
    let gen_skin = (gen_code << 1) | skin_code;
    let cksum = 0x08u8.wrapping_add(gen_skin).wrapping_add(dur_code);
    vec![0xFF, 0x08, gen_skin, dur_code, cksum]
}

fn prefs(duration: Duration, generation: Generation, skin: SkinSensitivity) -> Preferences {
    Preferences {
        duration,
        generation,
        skin_sensitivity: skin,
    }
}

// ---------------------------------------------------------------------------
// Tests – mirrors the original Python parametrize cases exactly
// ---------------------------------------------------------------------------

#[test]
fn child_sensitive_short() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Short,
            Generation::Child,
            SkinSensitivity::Sensitive
        )),
        [0xFF, 0x08, 0x00, 0x00, 0x08]
    );
}

#[test]
fn child_sensitive_medium() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Medium,
            Generation::Child,
            SkinSensitivity::Sensitive
        )),
        [0xFF, 0x08, 0x00, 0x01, 0x09]
    );
}

#[test]
fn child_sensitive_long() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Long,
            Generation::Child,
            SkinSensitivity::Sensitive
        )),
        [0xFF, 0x08, 0x00, 0x02, 0x0A]
    );
}

#[test]
fn adult_sensitive_short() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Short,
            Generation::Adult,
            SkinSensitivity::Sensitive
        )),
        [0xFF, 0x08, 0x02, 0x00, 0x0A]
    );
}

#[test]
fn adult_sensitive_medium() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Medium,
            Generation::Adult,
            SkinSensitivity::Sensitive
        )),
        [0xFF, 0x08, 0x02, 0x01, 0x0B]
    );
}

#[test]
fn adult_sensitive_long() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Long,
            Generation::Adult,
            SkinSensitivity::Sensitive
        )),
        [0xFF, 0x08, 0x02, 0x02, 0x0C]
    );
}

#[test]
fn child_regular_short() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Short,
            Generation::Child,
            SkinSensitivity::Regular
        )),
        [0xFF, 0x08, 0x01, 0x00, 0x09]
    );
}

#[test]
fn child_regular_medium() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Medium,
            Generation::Child,
            SkinSensitivity::Regular
        )),
        [0xFF, 0x08, 0x01, 0x01, 0x0A]
    );
}

#[test]
fn child_regular_long() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Long,
            Generation::Child,
            SkinSensitivity::Regular
        )),
        [0xFF, 0x08, 0x01, 0x02, 0x0B]
    );
}

#[test]
fn adult_regular_short() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Short,
            Generation::Adult,
            SkinSensitivity::Regular
        )),
        [0xFF, 0x08, 0x03, 0x00, 0x0B]
    );
}

#[test]
fn adult_regular_medium() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Medium,
            Generation::Adult,
            SkinSensitivity::Regular
        )),
        [0xFF, 0x08, 0x03, 0x01, 0x0C]
    );
}

#[test]
fn adult_regular_long() {
    assert_eq!(
        expected_request(&prefs(
            Duration::Long,
            Generation::Adult,
            SkinSensitivity::Regular
        )),
        [0xFF, 0x08, 0x03, 0x02, 0x0D]
    );
}

#[test]
fn default_preferences_are_child_sensitive_short() {
    assert_eq!(
        expected_request(&Preferences::default()),
        [0xFF, 0x08, 0x00, 0x00, 0x08],
        "Default preferences must encode as Child + Sensitive + Short"
    );
}

// ---------------------------------------------------------------------------
// Mock-backend tests: drive HeatItDevice through the async BulkTransferDevice
// trait without real hardware.
// ---------------------------------------------------------------------------

/// Records all requests and replies with scripted responses (12 zero bytes
/// once the script runs out).
struct MockBackend {
    requests: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: VecDeque<Vec<u8>>,
}

impl MockBackend {
    fn new(responses: impl IntoIterator<Item = Vec<u8>>) -> (Self, Arc<Mutex<Vec<Vec<u8>>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                requests: Arc::clone(&requests),
                responses: responses.into_iter().collect(),
            },
            requests,
        )
    }
}

#[async_trait]
impl BulkTransferDevice for MockBackend {
    async fn bulk_transfer(&mut self, request: &[u8]) -> heatr::error::Result<Vec<u8>> {
        self.requests.lock().unwrap().push(request.to_vec());
        Ok(self.responses.pop_front().unwrap_or_else(|| vec![0u8; 12]))
    }

    fn product_name(&self) -> Option<String> {
        Some("mock".into())
    }

    fn serial_number(&self) -> Option<String> {
        None
    }
}

/// Builds a 12-byte GET_STATUS response with the given flags/phase/temp.
fn status_response(flags: u8, phase: u8, temperature: u8) -> Vec<u8> {
    let mut r = vec![0u8; 12];
    r[3] = temperature;
    r[4] = flags;
    r[5] = phase;
    r
}

const STOP_HEATING: &[u8] = &[0xFF, 0x18, 0x18];

#[test]
fn msg_start_heating_sends_expected_bytes() {
    pollster::block_on(async {
        for duration in [Duration::Short, Duration::Medium, Duration::Long] {
            for generation in [Generation::Child, Generation::Adult] {
                for skin in [SkinSensitivity::Sensitive, SkinSensitivity::Regular] {
                    let p = prefs(duration, generation, skin);
                    let (backend, requests) = MockBackend::new([]);
                    let mut device = HeatItDevice::new(Box::new(backend));
                    device.msg_start_heating(&p).await.unwrap();
                    assert_eq!(
                        requests.lock().unwrap().as_slice(),
                        &[expected_request(&p)],
                        "wire bytes for {p}"
                    );
                }
            }
        }
    });
}

#[test]
fn monitor_yields_statuses_until_done_without_auto_stop() {
    pollster::block_on(async {
        let poll_echo = vec![0u8; 12];
        let (backend, requests) = MockBackend::new([
            status_response(0x80, 0x01, 0x50), // heating
            poll_echo.clone(),
            status_response(0x00, 0x01, 0xE1), // applying
            poll_echo.clone(),
            status_response(0x00, 0x00, 0x46), // done
            poll_echo.clone(),
        ]);
        let mut device = HeatItDevice::new(Box::new(backend));

        let mut phases = Vec::new();
        {
            let stream = device.monitor();
            pin_mut!(stream);
            while let Some(status) = stream.next().await {
                phases.push(status.unwrap().phase);
            }
        }

        assert_eq!(
            phases,
            [
                HeatingPhase::Heating,
                HeatingPhase::Applying,
                HeatingPhase::Done
            ]
        );
        // The stream itself must not send STOP_HEATING; that is the caller's
        // responsibility.
        assert!(
            !requests.lock().unwrap().iter().any(|r| r == STOP_HEATING),
            "monitor must not auto-send STOP_HEATING"
        );
    });
}

#[test]
fn cancelling_monitor_by_dropping_stream_allows_stop_heating() {
    pollster::block_on(async {
        let poll_echo = vec![0u8; 12];
        let (backend, requests) = MockBackend::new([
            status_response(0x80, 0x01, 0x50), // heating
            poll_echo.clone(),
            // remaining script unused: stream is dropped after one item
        ]);
        let mut device = HeatItDevice::new(Box::new(backend));

        {
            let stream = device.monitor();
            pin_mut!(stream);
            let first = stream.next().await.unwrap().unwrap();
            assert_eq!(first.phase, HeatingPhase::Heating);
            // Drop the stream mid-session (user pressed Stop).
        }

        device.stop_heating().await.unwrap();
        assert_eq!(
            requests.lock().unwrap().last().map(Vec::as_slice),
            Some(STOP_HEATING),
            "STOP_HEATING must be the last command after cancellation"
        );
    });
}

#[test]
fn poll_status_pairs_get_status_with_poll() {
    pollster::block_on(async {
        let (backend, requests) =
            MockBackend::new([status_response(0x80, 0x01, 0x50), vec![0u8; 12]]);
        let mut device = HeatItDevice::new(Box::new(backend));

        let status = device.poll_status().await.unwrap();
        assert_eq!(status.phase, HeatingPhase::Heating);
        assert_eq!(status.temperature, 0x50);

        let requests = requests.lock().unwrap();
        assert_eq!(
            requests.as_slice(),
            &[vec![0xFF, 0x02, 0x02], vec![0xFF, 0x32, 0x32]],
            "GET_STATUS must always be followed by POLL"
        );
    });
}
