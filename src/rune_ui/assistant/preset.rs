//! "Set me up like Denis" in the pane: the owner's settings as one plan
//! (`assistant::setup::preset`), no model asked, and only where it is
//! offered (`preset::offered`: Russian recognition). Once it is applied or
//! put off, a second step shows his shortcuts as the shortcuts card
//! (`keys`): a recorder per row, "Use these" saving them with Undo, "Keep
//! mine" leaving them. A message typed meanwhile is a question like any
//! other.

use super::super::Ui;
use crate::assistant::setup::flow::{Event, Flow};
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

#[cfg(test)]
mod tests {
    use super::super::ask::decided;
    use super::super::changes::{self, At};
    use super::*;
    use crate::assistant::changes::CardKind;
    use crate::assistant::setup::flow::{self, SETTINGS, SHORTCUTS, Step};
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
        assert_eq!(second.cards[0].kind, CardKind::Shortcuts);
        let keys: Vec<&str> = second.cards[0]
            .items
            .iter()
            .map(|item| item.key.as_str())
            .collect();
        // The mode is always a row; Denis's Mac key only where it is taken.
        assert!(keys.contains(&"shortcut_mode"), "{keys:?}");
        if cfg!(target_os = "macos") {
            assert!(keys.contains(&"dictation_hotkey"), "{keys:?}");
        }
        // Deciding again doesn't ask twice.
        decided(&mut ui, at);
        assert_eq!(turns(&ui).len(), 2);
        // Use these is the card's save: the step is done, nothing pushed.
        let shortcuts = At {
            conversation: at.conversation,
            turn: 1,
            card: 0,
        };
        changes::apply(&mut ui, shortcuts);
        assert!(matches!(
            turns(&ui)[1].cards[0].decision,
            crate::assistant::changes::Decision::Applied { .. }
        ));
        assert_eq!(turns(&ui).len(), 2);
        let open = ui.assistant.conversation().unwrap();
        assert_eq!(
            flow::step(&open.turns, ui.assistant.store.setups()),
            Step::Done
        );
    }
}
