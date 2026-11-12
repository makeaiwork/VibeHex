//! Instructions a card would save, shown line by line against the ones
//! saved now: removed lines on a soft danger tint, added ones on a soft
//! success tint, a few unchanged lines around them. Edit first opens the
//! text in the card itself, saved through the same check as the model's.

use rune::{El, col, row, text};
use rune_kit::TextArea;
use rune_kit::diff::{FileDiff, LineKind};

use super::super::Ui;
use super::super::controls::Paint;
use super::card::{Look, push};
use super::changes::{self as act, At};
use crate::desktop::i18n::tr;

/// Unchanged lines kept around each change.
const CONTEXT: u32 = 2;

fn diff(old: &str, new: &str) -> FileDiff {
    FileDiff::from_texts_with_context("instructions", old, new, CONTEXT)
}

/// How many lines the change touches, as its chip says.
pub(super) fn changed(old: &str, new: &str) -> usize {
    let diff = diff(old, new);
    diff.additions().max(diff.deletions())
}

/// The lines changed, or the text while it is edited.
pub(super) fn lines(ui: &Ui, p: Paint, at: At, old: &str, new: &str) -> El {
    if ui.assistant.editing != Some(at) {
        return view(p, old, new);
    }
    row().key("diff").pl(38.0).child(
        TextArea::new(&ui.assistant.edit)
            .label(tr("Instructions"))
            .rows(8)
            .width(None)
            .on_edit(|ui: &mut Ui| &mut ui.assistant.edit)
            .build(p.k)
            .key("edit")
            .grow(1.0)
            .shrink()
            .min_w(0.0),
    )
}

/// Save instructions, then Edit first and Not now; while editing, Save
/// instructions saves the edited text and Cancel edit goes back.
pub(super) fn buttons(ui: &Ui, p: Paint, at: At, look: Look) -> Vec<El> {
    let save = push(p, "apply", tr("Save instructions"), look, move |ui, _| {
        act::save_edit(ui, at)
    });
    if ui.assistant.editing == Some(at) {
        return vec![
            save,
            push(
                p,
                "cancel-edit",
                tr("Cancel edit"),
                Look::Secondary,
                |ui, _| act::cancel_edit(ui),
            ),
        ];
    }
    vec![
        save,
        push(
            p,
            "edit",
            tr("Edit first"),
            Look::Secondary,
            move |ui, _| act::edit(ui, at),
        ),
        push(
            p,
            "not-now",
            tr("Not now"),
            Look::Secondary,
            move |ui, _| act::decline(ui, at),
        ),
    ]
}

/// The lines, in a hairline box, starting where the card's words start.
fn view(p: Paint, old: &str, new: &str) -> El {
    let c = &p.k.c;
    let font = rune::TextStyle {
        size: 11.5,
        line_height: 18.0,
        ..p.k.ty.mono
    };
    let mut lines = col().key("lines");
    let mut index = 0;
    for (number, hunk) in diff(old, new).hunks.iter().enumerate() {
        if number > 0 {
            lines = lines.child(
                text("…")
                    .key(format!("gap{number}"))
                    .font(font)
                    .color(p.faint())
                    .px(10.0),
            );
        }
        for line in &hunk.lines {
            let (mark, fill, ink) = match line.kind {
                LineKind::Context => (" ", None, p.muted()),
                LineKind::Removed => ("−", Some(p.k.alpha(p.danger(), 0.12)), p.danger()),
                LineKind::Added => ("+", Some(p.k.alpha(p.success(), 0.12)), p.success()),
            };
            let mut shown = row()
                .key(format!("line{index}"))
                .px(10.0)
                .py(2.0)
                .gap(6.0)
                .items_start()
                .child(text(mark).key("mark").font(font).color(ink))
                .child(
                    text(line.text.clone())
                        .key("text")
                        .font(font)
                        .color(ink)
                        .wrap()
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0),
                );
            if let Some(fill) = fill {
                shown = shown.fill(fill);
            }
            lines = lines.child(shown);
            index += 1;
        }
    }
    row().key("diff").pl(38.0).child(
        lines
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .py(4.0)
            .radius(p.k.radius.md)
            .border(p.hair, c.border_weak)
            .clip_soft(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chip_counts_the_lines_that_change() {
        let old = "Fix punctuation.\nKeep English terms as said.\nAdd nothing.";
        let new = "Fix punctuation.\nKeep English terms in Latin script.\nNever translate code.\nAdd nothing.";
        assert_eq!(changed(old, new), 2);
        assert_eq!(changed(old, old), 0);
    }
}
