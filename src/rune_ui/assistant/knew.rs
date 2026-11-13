//! The line over an answer that folds how it was made: "Read 4 meetings
//! and 1 file in 9 s ›", a click listing the steps, and under them, for an
//! answer of this session, "What Assistant knew ›": what its model was
//! told of this install (`facts::text::knew`), as plain text. The record
//! lives with the turn in memory only, so an answer from an earlier
//! session has no such line, and a guide answer without one has no fold.

use rune::{El, col, el, icon, row, text};
use rune_theme_opencode::icons as theme;

use super::super::Ui;
use super::super::controls::Paint;
use super::recall;
use crate::assistant::agent::words;
use crate::assistant::store::Turn;
use crate::desktop::i18n::tr;

/// The width a step's mark and gap take, so the line sits on their words.
const UNDER_WORDS: f32 = 22.0;

/// The steps of the answer at `index` folded, which a click opens. None
/// for an answer that searched nothing and kept no record.
pub(super) fn folded(
    ui: &Ui,
    p: Paint,
    conversation: u32,
    index: usize,
    turn: &Turn,
) -> Option<El> {
    let at = (conversation, index);
    // A router turn's search and an agent turn's trace, as one list.
    let trace = words::of(&turn.steps, &turn.trace);
    let knew = !turn.knew.trim().is_empty();
    if trace.is_empty() && turn.web.is_empty() && !knew {
        return None;
    }
    let line = recall::summary((&trace, false), Some(turn), turn.seconds);
    let open = ui.assistant.unfolded.contains(&at);
    let mut list = recall::trace_lines(p, &trace);
    list.extend(super::web::steps(p, turn));
    if knew {
        let shown = ui.assistant.knew_open.contains(&at);
        list.push(record(p, at, &turn.knew, shown));
    }
    Some(recall::fold(
        p,
        line,
        open,
        move |ui: &mut Ui| flip(&mut ui.assistant.unfolded, at),
        list,
    ))
}

fn flip(open: &mut Vec<(u32, usize)>, at: (u32, usize)) {
    match open.iter().position(|known| *known == at) {
        Some(index) => {
            open.remove(index);
        }
        None => open.push(at),
    }
}

/// "What Assistant knew ›", and under it while `shown` the record, a line
/// each.
fn record(p: Paint, at: (u32, usize), knew: &str, shown: bool) -> El {
    let c = &p.k.c;
    let words = tr("What Assistant knew");
    let toggle = row()
        .key("knew-toggle")
        .role("button")
        .label(words)
        .checked(shown)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .self_start()
        .gap(4.0)
        .center_y()
        .radius(p.k.radius.sm)
        .child(
            text(words.to_owned())
                .key("words")
                .font(p.k.ty.small)
                .color(p.faint()),
        )
        .child(
            icon(
                if shown {
                    &theme::CHEVRON_DOWN
                } else {
                    &theme::CHEVRON_RIGHT
                },
                12.0,
            )
            .key("chevron")
            .color(c.icon_muted),
        )
        .on_press(move |ui: &mut Ui| flip(&mut ui.assistant.knew_open, at));
    let mut record = col().key("knew").gap(6.0).pl(UNDER_WORDS).child(toggle);
    if shown {
        let mut lines = col().key("lines").gap(3.0);
        for (index, line) in knew.lines().enumerate() {
            lines = lines.child(if line.trim().is_empty() {
                el().key(format!("gap{index}")).h(4.0)
            } else {
                text(line.to_owned())
                    .key(format!("line{index}"))
                    .font(p.k.ty.small)
                    .color(p.muted())
                    .wrap_anywhere()
                    .selectable(true)
            });
        }
        record = record.child(lines);
    }
    record
}
