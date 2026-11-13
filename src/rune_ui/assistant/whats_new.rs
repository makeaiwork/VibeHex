//! After an update, a way into what changed: for two weeks a new
//! conversation offers "Updated to <version>" over "Set VibeHex up for
//! me", and a press asks what's new since the version run before, from the
//! release notes (`assistant::releases`). Asked once, it goes. There is no
//! first-launch bubble on the face to show it in; the Start screen is
//! where the Assistant's other ways in are.
//!
//! What leads there is a mark: after an update the sidebar's Assistant
//! row (and the folded rail's face) carries a dot until Assistant is first
//! opened, which lands on a new conversation's start page, this row and
//! "Set VibeHex up for me" together. Then the mark is gone for good for
//! that version (`releases::history::note_opened`).

use rune::{El, col, icon, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::icons;
use super::super::sidebar::Pane;
use crate::assistant::intent;
use crate::assistant::releases::history;
use crate::desktop::i18n::{tr, tr_fill};

/// The question a press asks, in the window's language.
const QUESTION: &str = "What's new since my last version?";

/// The version updated to, while it hasn't been asked about since.
fn offered(ui: &Ui) -> Option<&str> {
    let (version, at) = ui.assistant.updated.as_ref()?;
    let asked = ui
        .assistant
        .store
        .conversations()
        .iter()
        .any(|conversation| {
            conversation.turns.iter().any(|turn| {
                turn.created_ms >= *at
                    && (turn.question == tr(QUESTION) || intent::releases(&turn.question).is_some())
            })
        });
    (!asked).then_some(version.as_str())
}

/// Reads, once a run, whether the sidebar marks Assistant: an update as
/// fresh as the Updated to row's, that Assistant wasn't opened on yet.
pub(super) fn look(ui: &mut Ui) {
    let pane = &mut ui.assistant;
    if !pane.mark_read {
        pane.mark_read = true;
        pane.mark = history::unopened(pane.now_ms());
    }
}

/// The version Assistant's sidebar row is marked for, while it shows.
pub(in super::super) fn marked(ui: &Ui) -> Option<&str> {
    ui.assistant
        .mark
        .as_deref()
        .filter(|_| Pane::Assistant.shown(ui))
}

/// Assistant opened while marked: it lands on a new conversation's start
/// page, and the mark goes for good. True when it was marked.
pub(super) fn opened(ui: &mut Ui) -> bool {
    if ui.pane != Pane::Assistant || ui.labs_open {
        return false;
    }
    let Some(version) = ui.assistant.mark.take() else {
        return false;
    };
    ui.assistant.open = None;
    if ui.live() {
        history::note_opened(&version);
    }
    true
}

/// "Updated to 3.3.0-vibe.7", what a press does, and a chevron.
pub(super) fn row_for(ui: &Ui, p: Paint) -> Option<El> {
    let version = offered(ui)?;
    let c = &p.k.c;
    let title = tr_fill("Updated to {}", version);
    Some(
        row()
            .key("whats-new")
            .role("button")
            .label(title.as_str())
            .focusable(true)
            .focus_ring(2.0, 0.0, c.border_focus)
            .gap(12.0)
            .px(14.0)
            .py(12.0)
            .center_y()
            .radius(p.k.radius.md)
            .fill(c.layer_01)
            .border(p.hair, c.border_base)
            .hover(c.hover)
            .press(c.pressed)
            .child(
                rune::el()
                    .key("tile")
                    .size(30.0, 30.0)
                    .center()
                    .radius(p.k.radius.sm)
                    .fill(c.layer_02)
                    .child(icon(&icons::SPARKLES, 15.0).key("glyph").color(p.accent())),
            )
            .child(
                col()
                    .key("words")
                    .grow(1.0)
                    .shrink()
                    .min_w(0.0)
                    .gap(2.0)
                    .child(
                        text(title.clone())
                            .key("title")
                            .font(p.k.ty.body_medium)
                            .color(p.text()),
                    )
                    .child(
                        text(tr("Ask me what's new, and what of it suits how you work"))
                            .key("lead")
                            .font(p.k.ty.small)
                            .color(p.muted())
                            .wrap(),
                    ),
            )
            .child(
                icon(&rune_theme_opencode::icons::CHEVRON_RIGHT, 14.0)
                    .key("chevron")
                    .color(c.icon_muted),
            )
            .on_press(|ui: &mut Ui| {
                let question =
                    crate::assistant::language::within(&ui.settings, || tr(QUESTION).to_owned());
                super::composer::ask_releases(ui, question);
            }),
    )
}
