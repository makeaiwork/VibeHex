//! The quick requests over a meeting's field, as Files draws its own:
//! Clean up transcript, Action items (Files' request, as Files › Settings
//! words it), and Recent, the requests last typed in Files or about a
//! meeting. A meeting's notes already hold its summary, so there is no
//! Summary chip.

use rune::{El, TextEdit, row};
use rune_kit::{Menu as KitMenu, MenuItem, Placement};

use super::super::controls::Paint;
use super::super::files::actions::chip_button;
use super::super::playback::{self, Of};
use super::super::{Menu, Ui};
use super::requests::{TIDY, ask};
use crate::desktop::i18n::tr;
use crate::files::store;

const RECENT_MENU: &str = "meeting-recent";

/// Files' Action items request: its chip's label and what it asks, in the
/// language Files' requests follow.
fn action_items(ui: &Ui) -> (String, String) {
    let files = &ui.settings.files;
    let (language, _) = files.prompt_language(&ui.settings.transcription);
    let kept = files
        .quick_prompts
        .iter()
        .find(|chip| chip.builtin_id() == Some("action_items"));
    match kept {
        Some(chip) => (chip.label.clone(), chip.resolved(language)),
        None => (
            tr("Action items").to_owned(),
            crate::app_settings::prompt_defaults::quick_default("action_items", language)
                .unwrap_or_default()
                .to_owned(),
        ),
    }
}

/// What the model is asked for `question` when it is a chip's name.
pub(super) fn prompt(ui: &Ui, question: &str) -> Option<String> {
    let (label, prompt) = action_items(ui);
    (question.trim() == label && !prompt.is_empty()).then_some(prompt)
}

/// The chips; they rest while an answer comes.
pub(super) fn chips(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let live = ui.meetings.requests.tasks.is_empty();
    let tidy = chip_button(p, "tidy", tr(TIDY), None, live, |ui: &mut Ui| {
        ask(ui, None, tr(TIDY).to_owned());
    });
    let (label, _) = action_items(ui);
    let asked = label.clone();
    let items = chip_button(p, "action-items", &label, None, live, move |ui: &mut Ui| {
        ask(ui, None, asked.clone());
    });
    let listened = ui.meetings.selected.clone().map(Of::Meeting);
    let chips = row()
        .key("chips")
        .gap(6.0)
        .center_y()
        .children(listened.as_ref().and_then(|of| playback::lead(ui, p, of)))
        .child(tidy)
        .child(items)
        .children(recent(ui, p, live, layer));
    // Play unfolds into the player in the chips' place.
    listened
        .and_then(|of| playback::unfolded(ui, p, &of, layer))
        .unwrap_or(chips)
}

/// Recent, while there are recent requests: one fills the field.
fn recent(ui: &Ui, p: Paint, live: bool, layer: &mut Vec<El>) -> Option<El> {
    if ui.files.recent.is_empty() {
        return None;
    }
    let open = live && ui.open == Some(Menu::Pick(RECENT_MENU));
    let chevron = if open {
        p.k.glyph.chevron_up
    } else {
        p.k.glyph.chevron_down
    };
    let trigger = chip_button(
        p,
        "recent",
        tr("Recent"),
        Some(chevron),
        live,
        |ui: &mut Ui| {
            let key = Menu::Pick(RECENT_MENU);
            ui.open = (ui.open != Some(key)).then_some(key);
        },
    );
    if !live {
        return Some(trigger);
    }
    let mut menu = KitMenu::new("meeting-recent-menu")
        .placement(Placement::TopStart)
        .gutter(4.0)
        .min_width(280.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .group_label(tr("Recent requests"));
    for prompt in &ui.files.recent {
        let chosen = prompt.clone();
        menu = menu.item(MenuItem::new(prompt.clone()).on_select(move |ui: &mut Ui| {
            ui.open = None;
            ui.meetings.requests.ask = TextEdit::new(chosen.clone());
        }));
    }
    if !ui.is_fixture() {
        menu = menu
            .separator()
            .item(MenuItem::new(tr("Clear recent")).on_select(|ui: &mut Ui| {
                ui.open = None;
                if let Err(error) = store::clear_recent() {
                    tracing::warn!(%error, "couldn't clear recent requests");
                }
                ui.files.recent.clear();
            }));
    }
    Some(menu.trigger(trigger).build(p.k, layer))
}

/// Keeps a typed question among the recent requests Files shares.
pub(super) fn remember(ui: &mut Ui, question: &str) {
    if ui.is_fixture() {
        return;
    }
    if let Err(error) = store::remember(question) {
        tracing::warn!(%error, "couldn't keep the request among the recent ones");
    }
    ui.files.recent = store::recent();
}
