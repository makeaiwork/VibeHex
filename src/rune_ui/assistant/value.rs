//! A value in a card: the chip a setting's value, a term, or a way it is
//! heard stands in, and from → to. A value that fits one line is a chip
//! as wide as its words. A longer one (instructions, a long model's name,
//! a long way a term is heard) wraps in a chip of one width, to three
//! lines; past that a quiet "Show all" unfolds it in place and "Show less"
//! folds it back.
//!
//! A chip never gives up room: what is beside it wraps instead, and a
//! pair too wide to stand side by side stands one under the other. So a
//! value is cut only by its three lines, with "Show all" under them.

use rune::text::{RichLayoutOptions, measure, measure_with};
use rune::{El, TextStyle, col, row, text};
use serde_json::Value;

use super::super::Ui;
use super::super::controls::Paint;
use super::card::label;
use super::changes::At;
use crate::desktop::i18n::tr;

/// A chip's most width, and the one width of a chip whose value wraps.
const WIDTH: f32 = 220.0;
const PAD: f32 = 7.0;
/// The lines a long value shows folded.
const LINES: usize = 3;

/// Which value of which card: its row, and its place in the row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) struct ValueAt {
    pub(super) at: At,
    pub(super) row: usize,
    pub(super) slot: Slot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum Slot {
    From,
    To,
    /// A plan's "Already set": the value now.
    Kept,
    /// A way a term was heard, and one it will be.
    Was(usize),
    New(usize),
}

impl At {
    pub(super) fn value(self, row: usize, slot: Slot) -> ValueAt {
        ValueAt {
            at: self,
            row,
            slot,
        }
    }
}

/// How many lines `label` takes in a chip of the one width.
fn lines(label: &str, font: TextStyle) -> usize {
    let room = WIDTH - 2.0 * PAD;
    let (_, line) = measure("X", &[], font, None);
    let wrap = RichLayoutOptions::new().wrap_anywhere();
    let (_, height) = measure_with(label, &[], font, Some(room), wrap);
    (height / line.max(1.0)).round() as usize
}

/// How wide `label`'s chip is.
fn width(p: Paint, label: &str) -> f32 {
    let (words, _) = measure(label, &[], p.k.ty.small, None);
    (words + 2.0 * PAD).min(WIDTH)
}

fn toggle(ui: &mut Ui, at: ValueAt) {
    let open = &mut ui.assistant.values_open;
    match open.iter().position(|known| *known == at) {
        Some(index) => {
            open.remove(index);
        }
        None => open.push(at),
    }
}

/// A chip's fill, border, and ink: the value now outlined, the new one on
/// a soft accent tint, one turned down faint.
fn looks(p: Paint, new: bool, struck: bool) -> (rune::Color, rune::Color, rune::Color) {
    let c = &p.k.c;
    if new {
        (
            p.k.alpha(p.accent(), 0.12),
            p.k.alpha(p.accent(), 0.4),
            p.accent(),
        )
    } else if struck {
        (c.layer_02, c.border_weak, p.faint())
    } else {
        (c.layer_02, c.border_weak, p.muted())
    }
}

/// A few words in a chip of one line: a state ("Signed out"), a count, or
/// a value known to be short.
pub(super) fn short(p: Paint, key: &str, label: &str, new: bool, struck: bool) -> El {
    let (fill, border, ink) = looks(p, new, struck);
    let mut words = text(label.to_owned())
        .key("label")
        .font(p.k.ty.small)
        .color(ink)
        .max_lines(1)
        .shrink();
    if struck {
        words = words.strike(0, label.len(), None);
    }
    row()
        .key(key.to_owned())
        .h(20.0)
        .px(PAD)
        .center_y()
        .max_w(WIDTH)
        .radius(p.k.radius.xs)
        .fill(fill)
        .border(p.hair, border)
        .child(words)
}

/// A value: the one now outlined, the new one on a soft accent tint, or
/// struck through when it was turned down. One that wraps takes the one
/// width and three lines, with "Show all" when there is more.
pub(super) fn chip(
    ui: &Ui,
    p: Paint,
    at: ValueAt,
    key: &str,
    label: &str,
    new: bool,
    struck: bool,
) -> El {
    let font = p.k.ty.small;
    let takes = lines(label, font);
    if takes <= 1 {
        return short(p, key, label, new, struck);
    }
    let c = &p.k.c;
    let (fill, border, ink) = looks(p, new, struck);
    let open = ui.assistant.values_open.contains(&at);
    let mut words = text(label.to_owned())
        .key("label")
        .font(font)
        .color(ink)
        .wrap_anywhere()
        .w_full();
    if !open {
        words = words.max_lines(LINES);
    }
    if struck {
        words = words.strike(0, label.len(), None);
    }
    let mut chip = col()
        .key(key.to_owned())
        .w(WIDTH)
        .px(PAD)
        .py(3.0)
        .gap(2.0)
        .radius(p.k.radius.xs)
        .fill(fill)
        .border(p.hair, border)
        .child(words);
    if takes > LINES {
        let says = if open {
            tr("Show less")
        } else {
            tr("Show all")
        };
        chip = chip.child(
            row()
                .key("all")
                .role("button")
                .label(says)
                .checked(open)
                .focusable(true)
                .focus_ring(2.0, 0.0, c.border_focus)
                .radius(p.k.radius.xs)
                .hover(c.hover)
                .child(text(says).key("says").font(font).color(p.text()))
                .on_press(move |ui: &mut Ui| toggle(ui, at)),
        );
    }
    chip
}

/// A change of one value, and how its pair of chips stands.
pub(super) struct Change<'a> {
    pub(super) key: &'a str,
    pub(super) from: &'a Value,
    pub(super) to: &'a Value,
    /// Turned down: the new value struck through.
    pub(super) struck: bool,
    /// The pair stands at its row's far edge, so a short value keeps to it
    /// when the two are one under the other.
    pub(super) end: bool,
    /// The most the two chips take side by side, in points: wider, and
    /// they stand one under the other, so the row's words keep their room.
    pub(super) beside: f32,
}

/// The room a pair has beside a row's name and reason.
pub(super) const BESIDE: f32 = 260.0;

/// From → to, side by side; one under the other when the two are wider
/// than the room beside the row's words.
pub(super) fn change(ui: &Ui, p: Paint, at: At, index: usize, change: Change) -> El {
    let Change {
        key,
        from,
        to,
        struck,
        end,
        beside,
    } = change;
    let (was, now) = (label(key, from), label(key, to));
    let apart = width(p, &was) + width(p, &now) > beside;
    let from = chip(
        ui,
        p,
        at.value(index, Slot::From),
        "from",
        &was,
        false,
        false,
    );
    let to = chip(
        ui,
        p,
        at.value(index, Slot::To),
        "to",
        &now,
        !struck,
        struck,
    );
    let arrow = |points: &str| {
        text(points.to_owned())
            .key("arrow")
            .font(p.k.ty.small)
            .color(p.faint())
    };
    if !apart {
        return row()
            .key("values")
            .gap(6.0)
            .center_y()
            .child(from)
            .child(arrow("→"))
            .child(to);
    }
    let pair = col().key("values").gap(2.0);
    let pair = if end { pair.items_end() } else { pair };
    pair.child(from).child(arrow("↓")).child(to)
}
