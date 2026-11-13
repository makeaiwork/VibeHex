//! Assistant in the sidebar: one row at the bottom, above the version and
//! the update check, and an icon in the folded rail. The face stands where
//! the other items have an icon. It hovers and presses like the items
//! above it, but its open pane shows without a fill (variant B of the
//! Selected board): the face and the word turn bright, the word medium, and
//! the pane's title says the rest. A fill there read as a separate bar
//! beside the version; the items at the top keep theirs. After an update
//! both carry a dot until Assistant opens (`whats_new::marked`).

use rune::{El, Tx, col, el, row, text};

use super::super::Ui;
use super::super::controls::{Paint, status_dot};
use super::super::sidebar::{NAV_ROW_GAP, Pane};
use super::face::face;
use super::whats_new::marked;
use crate::desktop::i18n::{tr, tr_fill};

const KEY: &str = "nav-assistant";
/// The face is larger than the 16 pt icons above it (the Face board's
/// `.asst`), in a taller row; its outline starts where their icons do and
/// the word lands 2 pt right of their labels, the nearest a 24 pt face
/// allows with room around it.
const FACE: f32 = 24.0;
const ROW_HEIGHT: f32 = 32.0;

/// Whether its pane shows now.
fn on(ui: &Ui) -> bool {
    ui.pane == Pane::Assistant && !ui.labs_open
}

/// Switch on pointer down, as the sidebar does; the press keeps keyboard
/// activation.
/// A press squishes the face.
fn select_on_down(ui: &mut Ui, drag: &mut rune::Drag, tx: &mut Tx) {
    if drag.phase == rune::Phase::Down {
        ui.assistant.face.squish(tx);
        ui.select_pane(Pane::Assistant, tx);
    }
}

/// The face as the sidebar shows it: reading while an answer comes.
fn glyph(ui: &Ui, p: Paint, on: bool, still: bool) -> El {
    let c = &p.k.c;
    let pane = &ui.assistant;
    face(
        "face",
        FACE,
        if on { c.icon_base } else { c.icon_muted },
        c.icon_base,
        pane.face.mood(pane.answering.is_some(), still),
    )
}

/// The sidebar row, in its slot: Assistant and its face.
pub(in super::super) fn item(ui: &Ui, p: Paint, still: bool) -> Option<El> {
    if !Pane::Assistant.shown(ui) {
        return None;
    }
    let c = &p.k.c;
    let on = on(ui);
    let item = row()
        .key(KEY)
        .role("button")
        .label(tr("Assistant"))
        .h(ROW_HEIGHT)
        .radius(7.0)
        // The other items: 6 + a 16 pt icon + 8 to the word, 30. Here the
        // hexagon draws 3 pt inside its box: 3 + 24 + 5, 32.
        .px(3.0)
        .gap(5.0)
        .center_y()
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .on_drag(select_on_down)
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.select_pane(Pane::Assistant, tx))
        // The slot's hover, drawn on the item.
        .child(
            el().key("hover")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radius(7.0)
                .fill(c.hover)
                .group_opacity(0.0, 1.0)
                .a11y_hidden(),
        )
        .child(glyph(ui, p, on, still))
        .child(
            text(tr("Assistant"))
                .key("label")
                .font(if on { p.k.ty.ui_medium } else { p.k.ty.ui })
                .color(if on { c.text_base } else { c.text_muted })
                .shrink()
                .truncate()
                .a11y_hidden(),
        );
    // After an update, a dot at the row's end until Assistant opens.
    let item = match marked(ui) {
        Some(version) => item
            .description(tr_fill("Updated to {}", version))
            .child(el().key("spacer").grow(1.0))
            .child(mark(p).mr(9.0)),
        None => item,
    };
    Some(
        col()
            .key(format!("{KEY}-slot"))
            // Assistive technology sees the item alone.
            .role("none")
            .pt(NAV_ROW_GAP / 2.0)
            .pb(NAV_ROW_GAP / 2.0)
            .hover_group()
            // Pointing at the row blinks the face once.
            .on_hover_change(|ui: &mut Ui, inside, tx: &mut Tx| {
                if inside {
                    ui.assistant.face.blink(tx);
                }
            })
            .on_drag(select_on_down)
            .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.select_pane(Pane::Assistant, tx))
            .child(item),
    )
}

/// The folded rail's button: the face, its name as a tooltip.
pub(in super::super) fn rail_item(
    ui: &Ui,
    p: Paint,
    still: bool,
    layer: &mut Vec<El>,
) -> Option<El> {
    if !Pane::Assistant.shown(ui) {
        return None;
    }
    let c = &p.k.c;
    let on = on(ui);
    let button = row()
        .key(KEY)
        .role("button")
        .label(tr("Assistant"))
        .size(36.0, 32.0)
        .center()
        .radius(7.0)
        .hover(c.hover)
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .on_drag(select_on_down)
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.select_pane(Pane::Assistant, tx))
        .on_hover_change(|ui: &mut Ui, inside, tx: &mut Tx| {
            if inside {
                ui.assistant.face.blink(tx);
                ui.sidebar_tip = Some(KEY);
            } else if ui.sidebar_tip == Some(KEY) {
                ui.sidebar_tip = None;
            }
        })
        .child(glyph(ui, p, on, still));
    let button = match marked(ui) {
        Some(version) => button
            .description(tr_fill("Updated to {}", version))
            .child(mark(p).pin(None, Some(4.0), Some(4.0), None)),
        None => button,
    };
    Some(
        rune_kit::Tooltip::new(format!("{KEY}-tip"))
            .trigger(button)
            .text(p.k, tr("Assistant"))
            .placement(rune_kit::Placement::Right)
            .open(ui.sidebar_tip == Some(KEY) && ui.open.is_none())
            .build(p.k, layer),
    )
}

/// The mark after an update: the accent dot an available update has.
fn mark(p: Paint) -> El {
    status_dot(p.accent()).key("mark")
}
