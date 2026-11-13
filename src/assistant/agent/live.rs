//! An agent turn as the pane sees it while it runs: each thing read lands
//! in the answering state's trace, what is being done now beside it, and
//! a recordings tool called before the person said whether their
//! recordings may be read parks the turn on the consent card.
//!
//! The card comes at most once for a conversation: a no closes its
//! recordings until VibeHex quits (or until the person allows them in
//! Assistant › Settings, which opens them everywhere). A card left
//! unanswered for [`CONSENT_WAIT`] lets the turn go on without them and
//! decides nothing, so the Assistant is never held for long by a card the
//! person may not even see: it shows only in its own conversation.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::access::{Consent, Live};
use super::trace::{Did, Doing};
use crate::assistant::recordings::{Allowed, Step};
use crate::assistant::worker;

/// How often a parked turn looks for the person's answer.
const LOOK_EVERY: Duration = Duration::from_millis(50);
/// How long a parked turn waits for the person's answer to the card. Then
/// the recordings call reads as not answered, the model answers without
/// them, and nothing is saved: while it waits no other conversation can
/// ask, so a card the person walked away from holds the Assistant for no
/// longer than this.
pub const CONSENT_WAIT: Duration = Duration::from_secs(180);

/// A test's own wait, shorter than [`CONSENT_WAIT`].
#[cfg(test)]
static TEST_WAIT: Mutex<Option<Duration>> = Mutex::new(None);

fn consent_wait() -> Duration {
    #[cfg(test)]
    if let Some(wait) = *TEST_WAIT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
    {
        return wait;
    }
    CONSENT_WAIT
}

/// Parked turns wait `wait` for the card until the guard drops.
#[cfg(test)]
pub fn waiting(wait: Duration) -> impl Drop {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            *TEST_WAIT
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        }
    }
    *TEST_WAIT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(wait);
    Reset
}

/// The person's answer to the card, until the parked turn takes it.
static DECIDED: Mutex<Option<Option<Allowed>>> = Mutex::new(None);
/// The conversations whose recordings the person said no to, this run of
/// the app.
static DECLINED: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn decided() -> std::sync::MutexGuard<'static, Option<Option<Allowed>>> {
    DECIDED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn declined_list() -> std::sync::MutexGuard<'static, Vec<u32>> {
    DECLINED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The person's answer to the card a parked turn shows: leave to read
/// (the pane saves `assistant.read_recordings` first), or none.
pub fn decide(allowed: Option<Allowed>) {
    *decided() = Some(allowed);
}

/// The person said no for `conversation` since VibeHex started.
pub fn declined(conversation: u32) -> bool {
    declined_list().contains(&conversation)
}

pub(super) fn decline(conversation: u32) {
    let mut declined = declined_list();
    if !declined.contains(&conversation) {
        declined.push(conversation);
    }
}

/// Forgets every no: a scenario starts as a fresh run of the app.
#[cfg(test)]
pub fn forget() {
    declined_list().clear();
    *decided() = None;
}

/// The pane and the person, as a turn's tools reach them.
pub(super) struct Pane<'a> {
    pub cancel: &'a AtomicBool,
    pub conversation: u32,
}

impl Live for Pane<'_> {
    /// Shows the card and waits: for an answer, for Stop, or for
    /// [`CONSENT_WAIT`] to pass. The wait is the person's time, not the
    /// answer's.
    fn consent(&self) -> Consent {
        *decided() = None;
        worker::update(|answering| {
            answering.asking = true;
            answering.doing = None;
        });
        let parked = Instant::now();
        let wait = consent_wait();
        let consent = loop {
            if self.cancel.load(Ordering::Acquire) {
                break Consent::Stopped;
            }
            // Taken in a statement of its own: the lock is never held
            // through the sleep, or the pane's answer would wait on it.
            let said = decided().take();
            match said {
                Some(Some(allowed)) => break Consent::Allowed(allowed),
                Some(None) => break Consent::Declined,
                None if parked.elapsed() >= wait => break Consent::Unanswered,
                None => std::thread::sleep(LOOK_EVERY),
            }
        };
        let waited = parked.elapsed();
        worker::update(|answering| {
            answering.asking = false;
            answering.started += waited;
        });
        if matches!(consent, Consent::Declined) {
            decline(self.conversation);
        }
        consent
    }

    fn step(&self, step: &Step) {
        worker::update(|answering| answering.steps.push(step.clone()));
    }
}

/// One more thing read: into the trace, the search's own steps folded
/// into it.
pub(super) fn did(did: &Did) {
    worker::update(|answering| {
        answering.steps.clear();
        answering.trace.push(did.clone());
    });
}

pub(super) fn doing(doing: Option<Doing>) {
    worker::update(|answering| answering.doing = doing);
}

/// The reply was a request to read: nothing of it shows.
pub(super) fn asked_to_read() {
    worker::update(|answering| {
        answering.draft.clear();
        answering.cards_coming = false;
    });
}
