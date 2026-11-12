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
use super::changes::At;
use crate::assistant::setup::preset;
use crate::assistant::store::Turn;
use crate::desktop::i18n::{tr, tr_fill, tr_window};

/// A preset turn's steps (`Turn::preset`).
pub(super) const SETTINGS: &str = "settings";
pub(super) const SHORTCUTS: &str = "shortcuts";
const SHORTCUTS_PLAN: &str = "shortcuts-plan";
const DONE: &str = "done";

/// Starts the preset: in the open conversation when `question` was typed
/// there, else in a new one.
pub(super) fn begin(ui: &mut Ui, question: String) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    // Made for Russian dictation: with any other recognition there is no
    // way in, and none is made here.
    if ui.assistant.answering.is_some() || !preset::offered(&ui.settings) {
        return;
    }
    let context = super::changes::context(ui);
    let planned = preset::plan(&preset::denis(), &context.settings, &context.state);
    let now = ui.assistant.now_ms();
    let pane = &mut ui.assistant;
    pane.consent = None;
    let id = match pane.conversation() {
        Some(conversation) if !question.is_empty() || conversation.turns.is_empty() => {
            conversation.id
        }
        _ => pane.store.start(
            tr("Set up like Denis"),
            pane.engine.take(),
            pane.effort.take(),
        ),
    };
    let mut answer = if planned.card.is_some() {
        tr("Here is how Denis has VibeHex set up. Untick what you don't need.").to_owned()
    } else {
        tr("Everything already matches how Denis has it.").to_owned()
    };
    if !planned.missing.is_empty() {
        let missing: Vec<&str> = planned.missing.iter().map(|name| tr(name)).collect();
        answer.push_str("\n\n");
        answer.push_str(&tr_fill("Not available here: {}", &missing.join(", ")));
    }
    let settled = planned.card.is_none();
    pane.store.push(
        id,
        Turn {
            question,
            answer,
            cards: planned.card.into_iter().collect(),
            preset: SETTINGS.into(),
            created_ms: now,
            ..Turn::default()
        },
    );
    pane.open = Some(id);
    pane.error = None;
    if settled {
        shortcuts(ui, id);
    }
}

/// A card was applied or put off: the preset's plan leads on to its
/// shortcuts.
pub(super) fn decided(ui: &mut Ui, at: At) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let Some(conversation) = ui.assistant.store.get(at.conversation) else {
        return;
    };
    let last = conversation.turns.len().saturating_sub(1);
    if at.turn == last && conversation.turns[at.turn].preset == SETTINGS {
        shortcuts(ui, at.conversation);
    }
}

/// The second step: the owner's shortcuts, and whether they suit.
fn shortcuts(ui: &mut Ui, id: u32) {
    let list = preset::shortcut_list(&preset::denis(), &ui.settings);
    if list.is_empty() {
        return;
    }
    let mut answer = tr("These are Denis's shortcuts — do they suit you?").to_owned();
    answer.push('\n');
    for (path, name, binding) in list {
        // Two named alike tell themselves apart by where they are.
        let name = if path.is_empty() {
            tr_window(&name).to_owned()
        } else if path.ends_with(name.as_str()) {
            tr_window(&path).to_owned()
        } else {
            format!("{} › {}", tr_window(&path), tr_window(&name))
        };
        answer.push_str(&format!("\n- {name}: {binding}"));
    }
    let now = ui.assistant.now_ms();
    ui.assistant.store.push(
        id,
        Turn {
            answer,
            preset: SHORTCUTS.into(),
            created_ms: now,
            ..Turn::default()
        },
    );
}

/// Under the shortcuts while they wait: Use these, Keep mine, and that a
/// key typed instead works too.
pub(super) fn view(ui: &Ui, p: Paint, conversation: u32, index: usize, turn: &Turn) -> Option<El> {
    let last = ui
        .assistant
        .store
        .get(conversation)
        .is_some_and(|open| open.turns.len() == index + 1);
    if turn.preset != SHORTCUTS || !last || ui.assistant.answering.is_some() {
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
fn use_these(ui: &mut Ui, id: u32) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let card = preset::shortcuts(&preset::denis(), &ui.settings);
    let answer = if card.is_some() {
        tr("Here they are. Untick any you'd rather keep as they are.")
    } else {
        tr("Your shortcuts already match Denis's.")
    };
    let now = ui.assistant.now_ms();
    ui.assistant.store.push(
        id,
        Turn {
            question: tr("Use these").into(),
            answer: answer.into(),
            cards: card.into_iter().collect(),
            preset: SHORTCUTS_PLAN.into(),
            created_ms: now,
            ..Turn::default()
        },
    );
}

fn keep_mine(ui: &mut Ui, id: u32) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let now = ui.assistant.now_ms();
    ui.assistant.store.push(
        id,
        Turn {
            question: tr("Keep mine").into(),
            answer: tr("Your shortcuts stay as they are.").into(),
            preset: DONE.into(),
            created_ms: now,
            ..Turn::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::super::changes;
    use super::*;

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
