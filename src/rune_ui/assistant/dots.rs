//! The three dots of a step still going on ("Writing the answer",
//! "Searching your recordings", "Read your settings…"): a gentle wave, the
//! bright dot moving a → b → c every [`STEP`] while any such step shows,
//! in Assistant, Files, and Meetings' Requests alike. They hold still
//! under Reduce Motion and in scenes, and nothing wakes the window while
//! no step is live.

use std::cell::Cell;
use std::time::{Duration, Instant};

use rune::Tx;

/// How long the bright dot stays on one dot.
pub(in super::super) const STEP: Duration = Duration::from_millis(380);

/// The wave's clock, shared by every live step line.
#[derive(Default)]
pub(in super::super) struct Dots {
    /// When the steps went live; `None` while none is.
    since: Option<Instant>,
    /// The bright dot: 0, 1 or 2.
    phase: u8,
    /// Reduce Motion, as the last frame saw it.
    still: Cell<bool>,
}

impl Dots {
    /// Follows whether a step is `live` at `now`: the phase, and how long
    /// until it moves next (`None`: nothing to wake for). True when the
    /// dots moved.
    pub(in super::super) fn tick(&mut self, live: bool, now: Instant) -> (bool, Option<Duration>) {
        if !live || self.still.get() {
            let moved = self.phase != 0;
            self.since = None;
            self.phase = 0;
            return (moved, None);
        }
        let since = *self.since.get_or_insert(now);
        let elapsed = now.saturating_duration_since(since);
        let step = STEP.as_millis();
        let steps = elapsed.as_millis() / step;
        let phase = (steps % 3) as u8;
        let next = Duration::from_millis((step - elapsed.as_millis() % step) as u64);
        let moved = phase != self.phase;
        self.phase = phase;
        (moved, Some(next))
    }

    /// [`Self::tick`] now, waking the window for the next move.
    pub(in super::super) fn poll(&mut self, live: bool, tx: &mut Tx) -> bool {
        let (moved, next) = self.tick(live, Instant::now());
        if let Some(next) = next {
            tx.wake_after(next.max(Duration::from_millis(10)));
        }
        moved
    }

    /// Notes Reduce Motion for the frame being drawn.
    pub(in super::super) fn hold(&self, reduced: bool) {
        self.still.set(reduced);
    }

    /// The bright dot for a step line; `None` draws the still look.
    pub(in super::super) fn phase(&self) -> Option<u8> {
        (self.since.is_some() && !self.still.get()).then_some(self.phase)
    }
}

/// Each dot's alpha: the bright one, the one it left, then the last; the
/// still look is the wave at rest on the first dot, fading right.
pub(in super::super) fn alphas(phase: Option<u8>) -> [f32; 3] {
    let Some(phase) = phase else {
        return [0.9, 0.6, 0.35];
    };
    let mut alphas = [0.0; 3];
    for (index, alpha) in alphas.iter_mut().enumerate() {
        *alpha = match (3 + phase as usize - index) % 3 {
            0 => 0.9,
            1 => 0.6,
            _ => 0.35,
        };
    }
    alphas
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bright dot moves a step at a time while a step is live, wakes
    /// for the next one, and stops when none is.
    #[test]
    fn the_wave_moves_while_a_step_is_live_and_stops_after() {
        let mut dots = Dots::default();
        let start = Instant::now();
        assert_eq!(dots.tick(true, start), (false, Some(STEP)));
        assert_eq!(dots.phase(), Some(0));
        let (moved, next) = dots.tick(true, start + STEP + Duration::from_millis(30));
        assert!(moved);
        assert_eq!(next, Some(STEP - Duration::from_millis(30)));
        assert_eq!(dots.phase(), Some(1));
        dots.tick(true, start + STEP * 2);
        assert_eq!(dots.phase(), Some(2));
        dots.tick(true, start + STEP * 3);
        assert_eq!(dots.phase(), Some(0));
        assert_eq!(alphas(Some(1)), [0.6, 0.9, 0.35]);
        // Nothing live: back at rest, nothing to wake for.
        dots.tick(true, start + STEP * 4);
        assert_eq!(dots.tick(false, start + STEP * 5), (true, None));
        assert_eq!(dots.phase(), None);
        assert_eq!(dots.tick(false, start + STEP * 6), (false, None));
    }

    /// Under Reduce Motion the dots keep today's still look and never wake
    /// the window.
    #[test]
    fn reduce_motion_holds_the_dots_still() {
        let mut dots = Dots::default();
        dots.hold(true);
        let start = Instant::now();
        assert_eq!(dots.tick(true, start), (false, None));
        assert_eq!(dots.tick(true, start + STEP * 2), (false, None));
        assert_eq!(dots.phase(), None);
        assert_eq!(alphas(dots.phase()), [0.9, 0.6, 0.35]);
    }
}
