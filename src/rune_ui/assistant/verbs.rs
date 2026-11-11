//! What a card's main button, its turned-down button, its saved status,
//! and its Show button say.

use serde_json::Value;

use super::super::Ui;
use super::card::label;
use crate::assistant::changes::names::{self, Home};
use crate::assistant::changes::{Card, CardKind};
use crate::desktop::i18n::{tr, tr_fill};

/// The main button: what pressing it does.
pub(super) fn verb(card: &Card, key: &str, to: &Value, also: usize) -> String {
    match &card.kind {
        CardKind::Prompt => return tr("Save instructions").to_owned(),
        CardKind::Choice { .. } => return tr_fill("Use {}", &label(key, to)),
        _ => {}
    }
    match (also, to) {
        (1, _) => tr("Change both").to_owned(),
        (2.., _) => tr("Change all").to_owned(),
        (_, Value::Bool(true)) => tr("Turn on").to_owned(),
        (_, Value::Bool(false)) => tr("Turn off").to_owned(),
        _ => tr("Change").to_owned(),
    }
}

/// The button on a card turned down: the change after all.
pub(super) fn anyway(card: &Card, to: &Value) -> &'static str {
    match (&card.kind, to) {
        (CardKind::Single, Value::Bool(true)) => tr("Turn on anyway"),
        (CardKind::Single, Value::Bool(false)) => tr("Turn off anyway"),
        _ => tr("Change anyway"),
    }
}

/// What a saved card says.
pub(super) fn applied_words(card: &Card, key: &str, to: &Value) -> String {
    if card.kind == CardKind::Prompt {
        return tr("Saved").to_owned();
    }
    match (key, to) {
        ("meetings.enabled", Value::Bool(true)) => {
            tr("Turned on, Meetings is in the sidebar").to_owned()
        }
        (_, Value::Bool(true)) => tr("Turned on").to_owned(),
        (_, Value::Bool(false)) => tr("Turned off").to_owned(),
        _ => tr_fill("Set to {}", &label(key, to)),
    }
}

/// Show in Labs, or in the setting's pane.
pub(super) fn show_label(key: &str, ui: &Ui) -> &'static str {
    match names::place(key, &ui.settings).map(|place| place.home) {
        Some(Home::Labs(_)) | None => tr("Show in Labs"),
        Some(Home::General) => tr("Show in General"),
        Some(Home::Output) => tr("Show in Output"),
        Some(Home::History) => tr("Show in History"),
        Some(Home::Meetings) => tr("Show in Meetings"),
        Some(Home::Annotate) => tr("Show in Annotate"),
        Some(Home::Files) => tr("Show in Files"),
    }
}
