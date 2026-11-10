//! Assistant's face: VibeHex's hexagon with two round eyes (face B). The
//! hexagon is a static icon; the eyes are elements over it, so they can
//! blink, read, and squish with the face.

use rune::{Color, El, Icon, el, icon};

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

/// The face `size` points wide: the hexagon in `outline`, the eyes in
/// `eyes`.
pub(super) fn face(key: &str, size: f32, outline: Color, eyes: Color) -> El {
    let unit = size / 24.0;
    let diameter = 2.0 * EYE_R * unit;
    let eye = |key: &str, x: f32| {
        el().key(key)
            .size(diameter, diameter)
            .radius(diameter / 2.0)
            .fill(eyes)
            .pin(
                Some(x * unit - diameter / 2.0),
                Some(EYES_Y * unit - diameter / 2.0),
                None,
                None,
            )
    };
    el().key(key.to_owned())
        .size(size, size)
        .child(icon(&HEXAGON, size).key("hexagon").color(outline))
        .child(
            el().key("eyes")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .child(eye("left", EYES_X[0]))
                .child(eye("right", EYES_X[1])),
        )
}
