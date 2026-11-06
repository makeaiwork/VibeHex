//! Play at the head of the chips, and the oval player it unfolds into.

use rune::{El, el, icon, row, text};
use rune_kit::{Menu as KitMenu, MenuItem, Placement};

use super::super::controls::Paint;
use super::super::{Menu, Ui};
use super::{Of, close, enabled, play, seek, set_speed, skip, take, toggle};
use crate::desktop::i18n::{tr, tr_fill};
use crate::files::playback::{SPEEDS, Status};
use crate::files::text::clock;

/// A triangle pointing right: Play, also Labs' row.
pub(in super::super) static PLAY_ICON: rune::Icon = rune::Icon {
    name: "hex-playback-play",
    view: 12.0,
    body: r#"<path d="M3.2 1.6v8.8l7.2-4.4z" fill="currentColor"/>"#,
};

static PAUSE_ICON: rune::Icon = rune::Icon {
    name: "hex-playback-pause",
    view: 12.0,
    body: r#"<rect x="2.2" y="1.8" width="2.6" height="8.4" rx=".8" fill="currentColor"/><rect x="7.2" y="1.8" width="2.6" height="8.4" rx=".8" fill="currentColor"/>"#,
};

/// A turning arrow around a 5: back five seconds.
static BACK_ICON: rune::Icon = rune::Icon {
    name: "hex-playback-back",
    view: 16.0,
    body: r#"<path d="M5.2 3.1A5.6 5.6 0 1 1 2.6 7.4" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><path d="M5.6 0.9 4.9 3.3l2.4.7" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/><path d="M9.5 6.2H7.6l-.3 1.9c.3-.2.6-.3 1-.3.8 0 1.3.5 1.3 1.3 0 .8-.6 1.4-1.4 1.4-.5 0-1-.2-1.2-.6" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

const SPEED_MENU: &str = "playback-speed";

/// The chips' light fill and outline, which Play and the player share.
fn chip_look(p: Paint) -> (rune::Color, rune::Color) {
    let c = &p.k.c;
    if p.k.dark {
        (
            c.layer_01.mix(c.layer_02, 0.5),
            c.border_base.mix(c.border_strong, 0.5),
        )
    } else {
        (c.layer_02, c.border_strong)
    }
}

/// The accent's quiet fill behind what plays.
pub(super) fn accent_fill(p: Paint, strength: f32) -> rune::Color {
    rune::theme::for_blend(p.k.c.text_accent.alpha(strength), p.k.c.bg_base)
}

fn outlined(p: Paint, element: El) -> El {
    let (fill, outline) = chip_look(p);
    element.fill(fill).box_shadow(rune::Shadow {
        spread: 0.5,
        color: outline,
        ..Default::default()
    })
}

/// Play, first among the chips, while the player is folded: round, or
/// with the time it stopped at once it played part of the way.
pub(in super::super) fn lead(ui: &Ui, p: Paint, of: &Of) -> Option<El> {
    if !enabled(ui) {
        return None;
    }
    let pane = &ui.playback;
    let ours = pane.belongs(of);
    if ours && pane.open {
        return None;
    }
    let c = &p.k.c;
    let at = if ours { pane.at_ms() } else { 0 };
    let label = if at > 0 {
        tr_fill("Play from {}", &clock(at))
    } else {
        tr("Listen to the recording").to_owned()
    };
    let shown = of.clone();
    let mut button = row()
        .key("listen")
        .role("button")
        .label(label)
        .h(24.0)
        .center_y()
        .radius(12.0)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .hover(c.hover)
        .press(c.pressed)
        .child(icon(&PLAY_ICON, 10.0).key("glyph").color(c.icon_base))
        .on_press(move |ui: &mut Ui| play(ui, &shown));
    button = if at > 0 {
        button
            .pl(8.0)
            .pr(9.0)
            .gap(6.0)
            .child(time(p, "at", at, c.text_base))
    } else {
        button.w(24.0).justify_center()
    };
    Some(outlined(p, button))
}

/// The time in figures of one width, so Play's pill keeps its size as the
/// seconds turn and the rail beside it stays still.
fn time(p: Paint, key: &str, ms: u64, color: rune::Color) -> El {
    text(clock(ms))
        .key(key.to_owned())
        .font(
            rune::TextStyle {
                size: 12.0,
                ..p.k.ty.ui_medium
            }
            .features("'tnum' 1"),
        )
        .color(color)
        .max_lines(1)
}

/// A quiet round 24 pt button.
fn round(p: Paint, key: &str, label: &str, glyph: &'static rune::Icon, size: f32) -> El {
    let c = &p.k.c;
    row()
        .key(key.to_owned())
        .role("button")
        .label(label)
        .size(24.0, 24.0)
        .center()
        .radius(12.0)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .hover(c.hover)
        .press(c.pressed)
        .child(icon(glyph, size).key("glyph").color(c.icon_muted))
}

/// The player in the chips' place, while it is unfolded for `of`.
pub(in super::super) fn unfolded(ui: &Ui, p: Paint, of: &Of, layer: &mut Vec<El>) -> Option<El> {
    let pane = &ui.playback;
    if !enabled(ui) || !pane.belongs(of) || !pane.open {
        return None;
    }
    Some(player(ui, p, of, layer, true))
}

/// The player always shown, with no ×: a meeting without the field.
pub(in super::super) fn standing(ui: &Ui, p: Paint, of: &Of, layer: &mut Vec<El>) -> Option<El> {
    enabled(ui).then(|| player(ui, p, of, layer, false))
}

fn player(ui: &Ui, p: Paint, of: &Of, layer: &mut Vec<El>, closable: bool) -> El {
    let pane = &ui.playback;
    let ours = pane.belongs(of);
    let c = &p.k.c;
    let playing = ours && pane.playing();
    let at = if ours { pane.at_ms() } else { 0 };
    let shown = of.clone();
    let toggle_button = row()
        .key("play-pause")
        .role("button")
        .label(if playing { tr("Pause") } else { tr("Play") })
        .h(24.0)
        .pl(8.0)
        .pr(9.0)
        .gap(6.0)
        .center_y()
        .radius(12.0)
        .fill(accent_fill(p, if p.k.dark { 0.16 } else { 0.12 }))
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .press(c.pressed)
        .child(
            icon(if playing { &PAUSE_ICON } else { &PLAY_ICON }, 10.0)
                .key("glyph")
                .color(c.text_accent),
        )
        .child(time(p, "at", at, c.text_accent))
        .on_press(move |ui: &mut Ui| toggle(ui, &shown));
    let middle = match pane.status().filter(|_| ours) {
        Some(Status::Failed(error)) => text(tr_fill("Couldn't play it: {}", &error))
            .key("failed")
            .font(rune::TextStyle {
                size: 12.0,
                ..p.k.ty.ui
            })
            .color(p.danger())
            .max_lines(1)
            .grow(1.0)
            .shrink(),
        _ => scrub(p, at, pane.duration_ms().filter(|_| ours)),
    };
    let length = pane.duration_ms().filter(|_| ours);
    let back = of.clone();
    let player = row()
        .key("player")
        .role("group")
        .label(tr("Player"))
        .h(32.0)
        .grow(1.0)
        .shrink()
        .pl(4.0)
        .pr(4.0)
        .gap(4.0)
        .center_y()
        .radius(16.0)
        .children(closable.then(|| {
            round(p, "close-player", tr("Close player"), p.k.glyph.close, 12.0)
                .on_press(|ui: &mut Ui| close(ui))
        }))
        .child(toggle_button.ml(if closable { 0.0 } else { 4.0 }))
        .child(
            round(p, "back", tr("Back 5 seconds"), &BACK_ICON, 16.0).on_press(
                move |ui: &mut Ui| {
                    take(ui, &back);
                    skip(ui, false);
                },
            ),
        )
        .child(middle.mx(4.0))
        .child(
            text(length.map_or_else(|| "--:--".to_owned(), clock))
                .key("length")
                .font(rune::TextStyle {
                    size: 12.0,
                    ..p.k.ty.ui
                })
                .color(p.faint())
                .max_lines(1),
        )
        .child(speed(ui, p, layer));
    row().key("chips").child(outlined(p, player)).w_full()
}

/// The whole recording as a line, filled to where it is; a press or drag
/// moves there.
fn scrub(p: Paint, at: u64, length: Option<u64>) -> El {
    let c = &p.k.c;
    let done = length
        .map_or(0.0, |length| at as f32 / length.max(1) as f32)
        .clamp(0.0, 1.0);
    let knob = 12.0;
    row()
        .key("scrub")
        .role("slider")
        .label(tr("Position"))
        .h(24.0)
        .grow(1.0)
        .shrink()
        .min_w(40.0)
        .center_y()
        .on_drag(
            move |ui: &mut Ui, drag: &mut rune::Drag, _: &mut rune::Tx| {
                let Some(length) = ui.playback.duration_ms() else {
                    return;
                };
                if drag.cancelled() {
                    return;
                }
                let fraction = ((drag.x - knob / 2.0) / (drag.w - knob).max(1.0)).clamp(0.0, 1.0);
                seek(ui, (fraction * length as f32) as u64);
            },
        )
        .child(
            el().key("done")
                .h(4.0)
                .grow(done)
                .radius(2.0)
                .fill(c.icon_accent),
        )
        .child(
            el().key("knob")
                .size(knob, knob)
                .radius(knob / 2.0)
                .fill(c.text_base),
        )
        .child(
            el().key("left")
                .h(4.0)
                .grow(1.0 - done)
                .radius(2.0)
                .fill(c.layer_03),
        )
}

/// The speed, as a quiet pill with its menu.
fn speed(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let current = ui.playback.speed();
    let open = ui.open == Some(Menu::Pick(SPEED_MENU));
    let mut menu = KitMenu::new("playback-speed-menu")
        .placement(Placement::TopEnd)
        .gutter(4.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .group_label(tr("Speed"));
    for speed in SPEEDS {
        menu = menu.item(
            MenuItem::new(speed_label(speed))
                .checked((speed - current).abs() < 0.01)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    set_speed(ui, speed);
                }),
        );
    }
    let chevron = if open {
        p.k.glyph.chevron_up
    } else {
        p.k.glyph.chevron_down
    };
    let trigger = row()
        .key("speed")
        .role("button")
        .label(tr("Speed"))
        .h(24.0)
        .pl(7.0)
        .pr(5.0)
        .gap(2.0)
        .center_y()
        .radius(12.0)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .hover(c.hover)
        .press(c.pressed)
        .child(
            text(speed_label(current))
                .key("label")
                .font(rune::TextStyle {
                    size: 12.0,
                    ..p.k.ty.ui
                })
                .color(p.muted()),
        )
        .child(icon(chevron, 11.0).key("chevron").color(c.icon_muted))
        .on_press(|ui: &mut Ui| {
            let key = Menu::Pick(SPEED_MENU);
            ui.open = (ui.open != Some(key)).then_some(key);
        });
    menu.trigger(trigger).build(p.k, layer)
}

/// `1×`, `1.25×`.
fn speed_label(speed: f32) -> String {
    let shown = format!("{speed:.2}");
    let shown = shown.trim_end_matches('0').trim_end_matches('.');
    format!("{shown}×")
}

#[cfg(test)]
mod tests {
    #[test]
    fn speeds_read_short() {
        assert_eq!(super::speed_label(1.0), "1×");
        assert_eq!(super::speed_label(1.25), "1.25×");
        assert_eq!(super::speed_label(0.75), "0.75×");
        assert_eq!(super::speed_label(1.5), "1.5×");
    }
}
