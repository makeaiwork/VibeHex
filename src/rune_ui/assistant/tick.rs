//! The checkbox a plan's and a vocabulary card's rows lead with. The
//! kit's draws its unchecked box in the faintest border, which reads as
//! nothing on a card: checked here is filled with the accent and a check
//! in the switch knob's ink, unchecked an empty box with a strong border,
//! both plain in light and dark.

use rune::{El, el, icon, row};

use super::super::Ui;
use super::super::controls::Paint;

/// A checkbox labelled `label` for assistive tech, `toggle` on a press.
pub(super) fn tick(p: Paint, checked: bool, label: &str, toggle: impl Fn(&mut Ui) + 'static) -> El {
    let c = &p.k.c;
    let mut control = el()
        .key("box")
        .size(16.0, 16.0)
        .center()
        .radius(p.k.radius.sm);
    control = if checked {
        control
            .fill(c.bg_accent)
            .border(1.0, c.bg_accent)
            .child(icon(p.k.glyph.check, 11.0).key("mark").color(c.knob_on))
    } else {
        control.fill(c.layer_01).border(1.0, c.border_strong)
    };
    row()
        .key("check")
        .role("checkbox")
        .label(label.to_owned())
        .checked(checked)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .child(control.a11y_hidden())
        .on_press(toggle)
}
