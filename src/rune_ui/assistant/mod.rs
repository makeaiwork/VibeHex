//! The Assistant pane: a chat that answers questions about VibeHex from
//! its guide (`crate::assistant`), and its settings. It sits at the bottom
//! of the sidebar on every OS, on by default and not in Labs, an exception
//! the owner chose. What reaches a model is the question, the conversation
//! so far, and guide text; never history, meetings, or files.

pub(super) mod face;
pub(super) mod fixture;
pub(super) mod nav;

use rune::{El, Tx, col, text};

use super::controls::{DESCRIPTION, Paint, segmented_tx};
use super::{Ui, lists};
use crate::desktop::i18n::tr;

/// Which half of the pane shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum View {
    #[default]
    Chat,
    Settings,
}

/// The pane's state.
#[derive(Default)]
pub(in super::super) struct AssistantPane {
    pub(super) view: View,
}

/// Chat or Settings, beside the title.
pub(super) fn view_switch(ui: &Ui, p: Paint) -> Option<El> {
    let selected = match ui.assistant.view {
        View::Chat => 0,
        View::Settings => 1,
    };
    Some(segmented_tx(
        p,
        "assistant-view",
        &[tr("Chat"), tr("Settings")],
        selected,
        |ui: &mut Ui, index: usize, tx: &mut Tx| {
            let view = if index == 0 {
                View::Chat
            } else {
                View::Settings
            };
            if ui.assistant.view != view {
                tx.settle();
                ui.open = None;
                ui.assistant.view = view;
            }
        },
    ))
}

/// The pane: its header actions and its body.
pub(super) fn pane(_ui: &Ui, p: Paint, _layer: &mut Vec<El>) -> (Vec<El>, El) {
    let body = col()
        .key("assistant-start")
        .px(32.0)
        .pt(24.0)
        .gap(6.0)
        .child(
            text(tr("Ask anything about VibeHex"))
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        );
    let body = body.child(
        text(tr(
            "Whether it can do something, how to set it up, which model to use",
        ))
        .key("lead")
        .font(DESCRIPTION)
        .color(p.muted())
        .wrap(),
    );
    (Vec::new(), lists::scroll_body("assistant-scroll", body))
}
