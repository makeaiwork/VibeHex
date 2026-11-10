//! Assistant's face: VibeHex's hexagon with two round eyes (face B). The
//! hexagon is a static icon; the eyes are elements over it, so they can
//! move with nothing round its edge spinning:
//!
//! - pointed at (the sidebar's row), it blinks once: the eyes close to a
//!   line and open, 0.34 s;
//! - pressed, it squishes and springs back past its size, 0.46 s;
//! - while an answer comes, it reads: the eyes drift right along a line
//!   over most of 2.6 s and come back, again and again.
//!
//! Each motion is a state the view draws (closed, squished, eyes right or
//! left) and the springs between them; `poll` ends a blink or a squish and
//! turns the reading eyes at their time. Under Reduce Motion nothing moves:
//! no blink, no squish, and a reading face rests its eyes to one side. A
//! scene draws the state at time zero.

use std::time::{Duration, Instant};

use rune::{Anim, Color, Curve, Easing, El, Icon, MotionIntent, Spring, Tx, el, icon};

/// The hexagon of VibeHex's logo, faintly filled with its own ink.
static HEXAGON: Icon = Icon {
    name: "hex-assistant-hexagon",
    view: 24.0,
    body: r#"<path d="M12 1.6 21 6.8v10.4L12 22.4 3 17.2V6.8Z" fill="currentColor" fill-opacity=".1" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/>"#,
};

/// The whole face as one static icon, where an element can't stand in.
pub(in super::super) static FACE: Icon = Icon {
    name: "hex-assistant-face",
    view: 24.0,
    body: r#"<path d="M12 1.6 21 6.8v10.4L12 22.4 3 17.2V6.8Z" fill="currentColor" fill-opacity=".1" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/><circle cx="9" cy="11.2" r="1.5" fill="currentColor"/><circle cx="15" cy="11.2" r="1.5" fill="currentColor"/>"#,
};

/// The face's grid: the eyes' centres and radius on the hexagon's 24 point
/// view.
const EYES_X: [f32; 2] = [9.0, 15.0];
const EYES_Y: f32 = 11.2;
const EYE_R: f32 = 1.5;
/// How far reading eyes drift either way, on the same grid.
const READ_SHIFT: f32 = 1.3;
/// A closed eye's height, as a share of an open one's.
const CLOSED: f32 = 0.1;

/// A blink closes for half its 0.34 s and opens for the other half.
const BLINK_HALF: Duration = Duration::from_millis(170);
/// A squish gathers for 0.14 s, then springs back over about 0.32 s.
const SQUISH_IN: Duration = Duration::from_millis(140);
const SQUISHED: f32 = 0.86;
/// One reading line: the eyes drift right for `READ_RIGHT`, then come back.
const READ_LINE: Duration = Duration::from_millis(2600);
const READ_RIGHT: Duration = Duration::from_millis(1900);

/// What the face is doing, for the view to draw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Mood {
    /// The eyes are closed (mid-blink).
    pub(super) closed: bool,
    /// The face is squished (mid-press).
    pub(super) squished: bool,
    /// Reading: whether the eyes head right.
    pub(super) reading: Option<bool>,
    /// Reduce Motion is on: nothing moves.
    pub(super) still: bool,
}

/// The face's moving state, shared by its places.
#[derive(Default)]
pub(in super::super) struct FaceState {
    closed_at: Option<Instant>,
    squished_at: Option<Instant>,
    /// While reading, the eyes head back left (the line's end); a line
    /// starts with them heading right.
    back: bool,
}

impl FaceState {
    /// One blink, from the pointer arriving.
    pub(super) fn blink(&mut self, tx: &mut Tx) {
        self.closed_at = Some(Instant::now());
        tx.wake_after(BLINK_HALF);
    }

    /// One squish, from a press.
    pub(super) fn squish(&mut self, tx: &mut Tx) {
        self.squished_at = Some(Instant::now());
        tx.wake_after(SQUISH_IN);
    }

    /// Ends a blink or a squish, and turns the reading eyes while an
    /// answer that started at `reading` comes. True when the face changed.
    pub(super) fn poll(&mut self, reading: Option<Instant>, tx: &mut Tx) -> bool {
        let mut changed = false;
        if self.closed_at.is_some_and(|at| at.elapsed() >= BLINK_HALF) {
            self.closed_at = None;
            changed = true;
        }
        if self.squished_at.is_some_and(|at| at.elapsed() >= SQUISH_IN) {
            self.squished_at = None;
            changed = true;
        }
        if let Some(started) = reading {
            let line = READ_LINE.as_secs_f64();
            let phase = started.elapsed().as_secs_f64() % line;
            let right = phase < READ_RIGHT.as_secs_f64();
            let next = if right {
                READ_RIGHT.as_secs_f64() - phase
            } else {
                line - phase
            };
            tx.wake_after(Duration::from_secs_f64(next.max(0.01)));
            if right == self.back {
                self.back = !right;
                changed = true;
            }
        }
        changed
    }

    /// What the face shows now; `reading` while an answer comes, `still`
    /// under Reduce Motion.
    pub(super) fn mood(&self, reading: bool, still: bool) -> Mood {
        Mood {
            closed: self.closed_at.is_some() && !still,
            squished: self.squished_at.is_some() && !still,
            reading: reading.then_some(!self.back || still),
            still,
        }
    }
}

/// The face `size` points wide: the hexagon in `outline`, the eyes in
/// `eyes`, doing what `mood` says.
pub(super) fn face(key: &str, size: f32, outline: Color, eyes: Color, mood: Mood) -> El {
    let unit = size / 24.0;
    let diameter = 2.0 * EYE_R * unit;
    let height = if mood.closed {
        diameter * CLOSED
    } else {
        diameter
    };
    let blink = if mood.still {
        Spring::NONE
    } else {
        Spring::new(BLINK_HALF.as_secs_f32(), 0.0)
    };
    let eye = |key: &str, x: f32| {
        el().key(key)
            .size(diameter, height)
            .radius(diameter / 2.0)
            .fill(eyes)
            .pin(
                Some(x * unit - diameter / 2.0),
                Some(EYES_Y * unit - height / 2.0),
                None,
                None,
            )
            .motion(blink)
    };
    // Reading eyes take most of a line to drift right and come back fast.
    let (shift, drift) = match mood.reading {
        Some(true) => (READ_SHIFT, READ_RIGHT.as_secs_f32() - 0.3),
        Some(false) => (-READ_SHIFT, 0.4),
        None => (0.0, 0.3),
    };
    let drift = if mood.still {
        Anim::INSTANT
    } else {
        Anim::new(Curve::tween(drift, Easing::EASE_IN_OUT))
    };
    let squish = if mood.still {
        Anim::INSTANT
    } else if mood.squished {
        Anim::new(Curve::tween(SQUISH_IN.as_secs_f32(), Easing::EASE_IN))
    } else {
        Anim::from(Spring::new(0.32, 0.45))
    };
    el().key(key.to_owned())
        .size(size, size)
        .scale(if mood.squished { SQUISHED } else { 1.0 })
        .scale_motion(squish)
        .motion_intent(MotionIntent::OVERSHOOT)
        .child(icon(&HEXAGON, size).key("hexagon").color(outline))
        .child(
            el().key("eyes")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .offset(shift * unit, 0.0)
                .offset_motion(drift)
                .child(eye("left", EYES_X[0]))
                .child(eye("right", EYES_X[1])),
        )
}
