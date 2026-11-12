//! "Set me up like Denis" in the pane: the owner's settings as one plan
//! (`assistant::setup::preset`), no model asked, and only where it is
//! offered (`preset::offered`: Russian recognition). Once it is applied or put
//! off, a second step shows his shortcuts and asks whether they suit:
//! "Use these" makes them a plan of their own, with Undo; "Keep mine"
//! leaves them; typing a key instead asks the model, whose change card
//! goes through the same check as any other.

use rune::{El, row, text};

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint};
use super::super::lists::small_button;
use crate::assistant::setup::flow::{self, Event, Flow, Step};
use crate::desktop::i18n::tr;

/// Starts the preset: in the open conversation when `question` was typed
/// there, else in a new one; nowhere when the setup has nothing to show
/// (it isn't offered here, or a model is answering).
pub(super) fn begin(ui: &mut Ui, question: String) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let pane = &ui.assistant;
    let open = pane
        .conversation()
        .filter(|open| !question.is_empty() || open.turns.is_empty());
    let (id, turns) = open.map_or_else(Default::default, |open| (open.id, open.turns.clone()));
    let flow = Flow::new(id, turns, pane.store.setups().to_vec());
    let (_, effects) = super::ask::on(ui, flow, Event::Preset { question });
    if effects.is_empty() {
        return;
    }
    let pane = &mut ui.assistant;
    pane.consent = None;
    let id = match id {
        0 => pane.store.start(
            tr("Set up like Denis"),
            pane.engine.take(),
            pane.effort.take(),
        ),
        id => id,
    };
    pane.open = Some(id);
    pane.error = None;
    super::ask::perform(ui, id, effects);
}

/// Under the shortcuts while they wait: Use these, Keep mine, and that a
/// key typed instead works too.
pub(super) fn view(ui: &Ui, p: Paint, conversation: u32, index: usize) -> Option<El> {
    let pane = &ui.assistant;
    let stands = pane
        .store
        .get(conversation)
        .map(|open| flow::step(&open.turns, pane.store.setups()));
    if stands != Some(Step::Shortcuts { turn: index }) || pane.answering.is_some() {
        return None;
    }
    Some(
        row()
            .key("preset-shortcuts")
            .gap(8.0)
            .center_y()
            .flex_wrap()
            .child(small_button(p, "use", tr("Use these"), move |ui, _| {
                use_these(ui, conversation)
            }))
            .child(small_button(p, "keep", tr("Keep mine"), move |ui, _| {
                keep_mine(ui, conversation)
            }))
            .child(
                text(tr("or type the key you'd like instead"))
                    .key("type")
                    .font(DESCRIPTION)
                    .color(p.faint())
                    .wrap()
                    .shrink(),
            ),
    )
}

/// The owner's shortcuts as a plan of their own.
pub(super) fn use_these(ui: &mut Ui, id: u32) {
    super::ask::send(ui, id, Event::UseShortcuts);
}

pub(super) fn keep_mine(ui: &mut Ui, id: u32) {
    super::ask::send(ui, id, Event::KeepShortcuts);
}

#[cfg(test)]
mod tests {
    use super::super::ask::decided;
    use super::super::changes::{self, At};
    use super::*;
    use crate::assistant::setup::flow::{SETTINGS, SHORTCUTS, SHORTCUTS_PLAN};
    use crate::assistant::store::Turn;

    fn turns(ui: &Ui) -> Vec<Turn> {
        ui.assistant.conversation().unwrap().turns.clone()
    }

    /// The plan first, every row checked but the keep-meetings time and
    /// no shortcut among them; once it is decided, the shortcuts, then
    /// their own plan.
    #[test]
    fn the_preset_plans_settings_then_offers_shortcuts() {
        let mut ui = super::super::super::scenes::mac_fixture();
        // Any recognition but Russian: no way in, and nothing started.
        for language in ["en", "auto"] {
            ui.settings.transcription.language = language.into();
            begin(&mut ui, "Set me up like Denis".into());
            assert!(ui.assistant.conversation().is_none(), "{language}");
        }
        ui.settings.transcription.language = "ru".into();
        begin(&mut ui, String::new());
        let first = &turns(&ui)[0];
        assert_eq!(first.preset, SETTINGS);
        let card = &first.cards[0];
        assert!(card.items.iter().all(|item| !item.key.ends_with("hotkey")));
        let at = At {
            conversation: ui.assistant.open.unwrap(),
            turn: 0,
            card: 0,
        };
        changes::decline(&mut ui, at);
        decided(&mut ui, at);
        let second = turns(&ui)[1].clone();
        assert_eq!(second.preset, SHORTCUTS);
        assert!(second.answer.contains("dictation") || second.answer.contains("Dictation"));
        // Deciding again doesn't ask twice.
        decided(&mut ui, at);
        assert_eq!(turns(&ui).len(), 2);
        let id = ui.assistant.open.unwrap();
        use_these(&mut ui, id);
        let third = turns(&ui)[2].clone();
        assert_eq!(third.preset, SHORTCUTS_PLAN);
        if let Some(card) = third.cards.first() {
            assert!(card.items.iter().all(|item| item.included));
        }
    }
}
