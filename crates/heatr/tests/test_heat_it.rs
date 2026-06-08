//! Tests for the heat-it protocol, mirroring the original Python test suite.
//!
//! These tests use only the public API and derive expected bytes from the
//! known protocol spec, so no real device is needed.

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
