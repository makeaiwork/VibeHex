//! Assistant in the sidebar: one row at the bottom, above the version and
//! the update check, behaving like the items above it (plain at rest, the
//! hover fill, the selected fill while its pane shows), and an icon in the
//! folded rail. The face stands where the other items have an icon.

use rune::{El, Tx, col, el, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::sidebar::{NAV_ROW_GAP, NAV_ROW_HEIGHT, Pane};
use super::face::face;
use crate::desktop::i18n::tr;

const KEY: &str = "nav-assistant";

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
        16.0,
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
    let mut item = row()
        .key(KEY)
        .role("button")
        .label(tr("Assistant"))
        .h(NAV_ROW_HEIGHT)
        .px(6.0)
        .gap(8.0)
        .center_y()
        .radius(p.k.radius.sm)
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .on_drag(select_on_down)
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.select_pane(Pane::Assistant, tx))
        // The slot's hover, drawn on the item.
        .child(
            el().key("hover")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radius(p.k.radius.sm)
                .fill(c.hover)
                .group_opacity(0.0, 1.0),
        )
        .child(glyph(ui, p, on, still))
        .child(
            text(tr("Assistant"))
                .key("label")
                .font(p.k.ty.ui)
                .color(if on { c.text_base } else { c.text_muted })
                .truncate(),
        );
    if on {
        item = item.fill(c.layer_02);
    }
    Some(
        col()
            .key(format!("{KEY}-slot"))
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
    let mut button = row()
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
    if on {
        button = button.fill(c.layer_02);
    }
    Some(
        rune_kit::Tooltip::new(format!("{KEY}-tip"))
            .trigger(button)
            .text(p.k, tr("Assistant"))
            .placement(rune_kit::Placement::Right)
            .open(ui.sidebar_tip == Some(KEY) && ui.open.is_none())
            .build(p.k, layer),
    )
}
