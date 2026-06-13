//! Runs a single heating session against a freshly connected device.
//!
//! A session is stateless: it connects, runs the init self-test, starts
//! heating, streams status until the cycle ends (or the user stops it), and
//! always sends `STOP_HEATING` on the way out. The whole lifecycle lives in
//! one async task that owns the device, so there is no shared-mutable-device
//! juggling — cancellation is a plain flag the task checks between polls.

use std::cell::Cell;
use std::pin::pin;
use std::rc::Rc;

use futures_util::StreamExt;
use heatr::{HeatingPhase, HeatingStatus, Preferences};

use crate::device;

/// How a session ended, for the UI to report.
pub enum Outcome {
    /// The treatment cycle ran to completion.
    Completed,
    /// The user pressed Stop.
    Stopped,
    /// Something went wrong (message is user-facing).
    Failed(String),
}

/// Runs a full session, calling `on_status` after every poll.
///
/// `stop` is shared with the UI; setting it requests cancellation, which takes
/// effect by the next poll boundary (≤ one poll interval).
pub async fn run(
    prefs: Preferences,
    stop: Rc<Cell<bool>>,
    mut on_status: impl FnMut(&HeatingStatus),
) -> Outcome {
    let mut device = match device::connect().await {
        Ok(device) => device,
        Err(e) => return Outcome::Failed(format!("Could not connect: {e}")),
    };

    if let Err(e) = device.self_test().await {
        return Outcome::Failed(format!("Initialization failed: {e}"));
    }
    if let Err(e) = device.start_with_preferences(&prefs).await {
        return Outcome::Failed(format!("Could not start heating: {e}"));
    }

    // Borrow the device for the monitor stream; drop it before stopping.
    let outcome = {
        let mut stream = pin!(device.monitor());
        loop {
            if stop.get() {
                break Outcome::Stopped;
            }
            match stream.next().await {
                Some(Ok(status)) => {
                    on_status(&status);
                    if status.phase == HeatingPhase::Done {
                        break Outcome::Completed;
                    }
                }
                Some(Err(e)) => break Outcome::Failed(format!("Device error: {e}")),
                None => break Outcome::Completed,
            }
        }
    };

    // Always try to leave the device idle, even on failure or stop.
    let _ = device.stop_heating().await;
    outcome
}

#[cfg(all(test, feature = "mock-device"))]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Drives the full pipeline against the mock device and cancels after the
    /// first poll, verifying we connect, start heating, observe a Heating
    /// status, and report `Stopped`.
    #[test]
    fn stop_after_first_status_yields_stopped() {
        pollster::block_on(async {
            let stop = Rc::new(Cell::new(false));
            let seen = RefCell::new(Vec::new());
            let stop_cb = Rc::clone(&stop);

            let outcome = run(Preferences::default(), Rc::clone(&stop), |status| {
                seen.borrow_mut().push(status.phase);
                stop_cb.set(true); // request stop after the first poll
            })
            .await;

            assert!(matches!(outcome, Outcome::Stopped));
            assert_eq!(seen.borrow().first(), Some(&HeatingPhase::Heating));
        });
    }

    /// Lets the mock cycle run to its natural end, verifying we report
    /// `Completed` and pass through Heating → Applying → Done with the
    /// temperature rising during the heating phase. Takes a few seconds
    /// because it waits out the simulated treatment.
    #[test]
    fn full_cycle_yields_completed() {
        pollster::block_on(async {
            let stop = Rc::new(Cell::new(false));
            let seen: RefCell<Vec<HeatingStatus>> = RefCell::new(Vec::new());

            let outcome = run(Preferences::default(), stop, |status| {
                seen.borrow_mut().push(*status);
            })
            .await;

            assert!(matches!(outcome, Outcome::Completed));

            let seen = seen.borrow();
            let phases: Vec<_> = seen.iter().map(|s| s.phase).collect();
            assert!(phases.contains(&HeatingPhase::Heating), "{phases:?}");
            assert!(phases.contains(&HeatingPhase::Applying), "{phases:?}");
            assert_eq!(phases.last(), Some(&HeatingPhase::Done), "{phases:?}");

            // Temperature climbs over the heating phase.
            let heating: Vec<u8> = seen
                .iter()
                .filter(|s| s.phase == HeatingPhase::Heating)
                .map(|s| s.temperature)
                .collect();
            assert!(
                heating.first() < heating.last(),
                "temperature should rise while heating: {heating:?}"
            );
        });
    }
}
