//! The shortcuts card: a row per shortcut whose feature is in use, each
//! with the recorder General uses (`rune_ui::assistant::keys`). It comes
//! in two places: the preset's second step, Denis's bindings proposed over
//! the person's, and an answer to a question about one's own shortcuts
//! ("help me set up my shortcuts", «какие у меня горячие клавиши»), the
//! bindings as they are (`intent::shortcuts` reads the question, `settle`
//! attaches the card, whatever model answered).
//!
//! A row is `included` exactly while its value differs from the one it
//! had when the card was made: proposed, or recorded since. The card saves
//! those rows, with what they need first, in one all-or-none
//! `control::settings::check`, the path every card's save takes, and Undo
//! puts back what it changed.

use serde_json::{Map, Value};

use super::{Card, CardKind, Item, current};
use crate::app_settings::AppSettings;
use crate::control::settings;

/// General's order: the dictation shortcut and how it triggers, the
/// shortcuts beside it, then the other panes'.
pub const KEYS: &[&str] = &[
    "dictation_hotkey",
    "shortcut_mode",
    "double_tap_lock",
    "double_tap_only",
    "cancel_hotkey",
    "second_dictation_hotkey",
    "terminal_paste_hotkey",
    "stop_hotkey",
    "capture.hotkey",
    "capture.replay_hotkey",
    "paste_last_hotkey",
    "vocabulary_add_hotkey",
    "raw_dictation_hotkey",
    "typed_text_hotkey",
    "screenshots.hotkey",
    "screenshots.record_hotkey",
];

/// The three settings behind General's one Shortcut mode choice: one row.
pub const MODE: [&str; 3] = ["shortcut_mode", "double_tap_lock", "double_tap_only"];

/// What the model is told when the app shows the card under its answer.
pub const NOTE: &str = "\n\nThe app shows a card under your answer listing the person's \
shortcuts, each with a field they click to record a new key, and a button that saves them. \
Write one short sentence that leads into it; never list the shortcuts or tell them to open \
Settings. If they named the key they want, write its change line as usual: it joins the card.";

/// What the model is told about the card `question` brings, if any.
pub fn note(question: &str) -> &'static str {
    crate::assistant::intent::shortcuts(question).map_or("", |_| NOTE)
}

/// Whether `key` is one of the card's.
pub fn is_key(key: &str) -> bool {
    KEYS.contains(&key)
}

/// Whether `key`'s shortcut at `value` does anything where every setting
/// reads as `after` says: its Labs feature on, Toggle for the cancel
/// shortcut (Hold has none, as General shows it), Annotate for its own.
/// The Shortcut mode is always in use. A shortcut not set counts as set,
/// so one whose feature is off is left out whatever it holds.
pub fn in_use(key: &str, value: &Value, after: &dyn Fn(&str) -> Option<Value>) -> bool {
    if MODE.contains(&key) {
        return true;
    }
    let set = match value {
        Value::Null => Value::Bool(true),
        value => value.clone(),
    };
    let mut needs = settings::requires(key, &set);
    match key {
        "cancel_hotkey" => needs.push(("shortcut_mode", Value::String("toggle".into()))),
        "capture.hotkey" => needs.push(("capture.enabled", Value::Bool(true))),
        _ => {}
    }
    needs.iter().all(|(needs, want)| {
        settings::unavailable(needs).is_none() && after(needs).as_ref() == Some(want)
    })
}

/// The card for `settings`: a row per shortcut this OS has and whose
/// feature is in use, among `only` when it names any, each from its value
/// now to the one `proposed` has for it (none: as it is). A proposal has
/// the rows it names, and the Shortcut mode's three with any of them. `beside` is
/// what comes with the proposal, such as the Labs switches a preset turns
/// on; with it a shortcut's feature counts as on. `None` without a row.
pub fn card(
    settings: &AppSettings,
    proposed: &Map<String, Value>,
    beside: &Map<String, Value>,
    only: &[&str],
) -> Option<Card> {
    let after = |key: &str| {
        proposed
            .get(key)
            .or_else(|| beside.get(key))
            .cloned()
            .or_else(|| current(settings, key))
    };
    // A preset's card has the shortcuts it names, an answer's those asked.
    let only: Vec<&str> = match proposed.is_empty() {
        true => only.to_vec(),
        false => proposed.keys().map(String::as_str).collect(),
    };
    let mode = only.iter().any(|key| MODE.contains(key));
    let mut items = Vec::new();
    for key in KEYS {
        let asked = only.is_empty() || only.contains(key) || (mode && MODE.contains(key));
        if !asked || settings::unavailable(key).is_some() {
            continue;
        }
        let Some(from) = current(settings, key) else {
            continue;
        };
        let to = proposed.get(*key).cloned().unwrap_or_else(|| from.clone());
        if !in_use(key, &to, &after) {
            continue;
        }
        let set = match &to {
            Value::Null => Value::Bool(true),
            to => to.clone(),
        };
        let requires = settings::missing(settings, key, &set);
        // A proposed key this OS can't take (Right Option off a Mac) is
        // left out with its row.
        if to != from {
            let mut values: Map<String, Value> = requires.iter().cloned().collect();
            values.insert((*key).to_owned(), to.clone());
            if settings::check(settings, &values, &crate::rune_ui::shortcut_rules()).is_err() {
                continue;
            }
        }
        items.push(Item {
            key: (*key).to_owned(),
            included: to != from,
            requires,
            from,
            to,
            ..Item::default()
        });
    }
    (!items.is_empty()).then(|| Card {
        kind: CardKind::Shortcuts,
        items,
        ..Card::default()
    })
}

/// Row `index` of `card` goes to `to`, recorded or picked; it counts as a
/// change while that differs from its value before.
pub fn set(card: &mut Card, index: usize, to: Value) {
    if let Some(item) = card.items.get_mut(index) {
        item.included = to != item.from;
        item.to = to;
    }
}

/// The settings as `card`'s save would make them, without row `skip`:
/// what a key recorded in that row must not collide with. A save that
/// wouldn't pass leaves the settings as they are.
pub fn after(card: &Card, settings: &AppSettings, skip: Option<usize>) -> AppSettings {
    let others = Card {
        items: card
            .items
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != skip)
            .map(|(_, item)| item.clone())
            .collect(),
        ..card.clone()
    };
    super::values(&others, settings)
        .ok()
        .and_then(|values| {
            settings::check(settings, &values, &crate::rune_ui::shortcut_rules()).ok()
        })
        .map_or_else(|| settings.clone(), |change| change.settings)
}

/// The rows the model proposed in its own cards, taken into `card`: a key
/// the person named ("set the dictation key to Right Command") is that
/// row's proposed value, and the model's card for it goes.
pub fn take(card: &mut Card, cards: &mut Vec<Card>) {
    cards.retain(|other| {
        let Some(item) = other.items.first() else {
            return true;
        };
        if other.items.len() != 1 || !is_key(&item.key) {
            return true;
        }
        let to = match &other.kind {
            CardKind::Choice {
                options, chosen, ..
            } => options.get(*chosen).cloned().unwrap_or(item.to.clone()),
            _ => item.to.clone(),
        };
        let Some(index) = card.items.iter().position(|row| row.key == item.key) else {
            return true;
        };
        set(card, index, to);
        card.items[index].requires.clone_from(&item.requires);
        false
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(card: &Card) -> Vec<&str> {
        card.items.iter().map(|item| item.key.as_str()).collect()
    }

    /// Only what is in use now: a Labs shortcut while its lab is on, the
    /// cancel shortcut in Toggle, the Shortcut mode always.
    #[test]
    fn the_card_lists_only_the_shortcuts_in_use() {
        let mut settings = AppSettings {
            shortcut_mode: crate::dictation::ShortcutMode::Hold,
            ..AppSettings::default()
        };
        let card = card(&settings, &Map::new(), &Map::new(), &[]).unwrap();
        let listed = keys(&card);
        for key in ["dictation_hotkey", "shortcut_mode", "paste_last_hotkey"] {
            assert!(listed.contains(&key), "{key}: {listed:?}");
        }
        for key in [
            "cancel_hotkey",
            "stop_hotkey",
            "second_dictation_hotkey",
            "vocabulary_add_hotkey",
            "terminal_paste_hotkey",
            "typed_text_hotkey",
            "screenshots.hotkey",
        ] {
            assert!(!listed.contains(&key), "{key}: {listed:?}");
        }
        // Nothing is proposed: no row is a change.
        assert!(card.items.iter().all(|item| !item.included));
        settings.shortcut_mode = crate::dictation::ShortcutMode::Toggle;
        settings.labs.vocabulary_quick_add = true;
        settings.labs.stop_shortcut = true;
        let card = super::card(&settings, &Map::new(), &Map::new(), &[]).unwrap();
        let listed = keys(&card);
        assert!(listed.contains(&"cancel_hotkey"), "{listed:?}");
        if cfg!(target_os = "macos") {
            assert!(listed.contains(&"vocabulary_add_hotkey"), "{listed:?}");
            assert!(listed.contains(&"stop_hotkey"), "{listed:?}");
        } else {
            assert!(!listed.contains(&"stop_hotkey"), "{listed:?}");
        }
    }

    /// A question about one shortcut gets its row; the mode, its three.
    #[test]
    fn a_card_narrowed_to_one_shortcut_has_its_row() {
        let settings = AppSettings::default();
        let card = card(&settings, &Map::new(), &Map::new(), &["paste_last_hotkey"]).unwrap();
        assert_eq!(keys(&card), ["paste_last_hotkey"]);
        let card = super::card(&settings, &Map::new(), &Map::new(), &["shortcut_mode"]).unwrap();
        assert_eq!(keys(&card), MODE);
    }

    /// The save takes the rows that changed, and nothing else.
    #[test]
    fn only_the_rows_changed_are_saved() {
        let settings = AppSettings::default();
        let mut card = card(&settings, &Map::new(), &Map::new(), &[]).unwrap();
        let paste = card
            .items
            .iter()
            .position(|item| item.key == "paste_last_hotkey")
            .unwrap();
        set(&mut card, paste, Value::String("control+shift+v".into()));
        let values = super::super::values(&card, &settings).unwrap();
        assert_eq!(values.keys().collect::<Vec<_>>(), ["paste_last_hotkey"]);
        // Recorded back as it was, it is no change.
        let from = card.items[paste].from.clone();
        set(&mut card, paste, from);
        assert!(super::super::values(&card, &settings).unwrap().is_empty());
    }

    /// The model's own card for a shortcut becomes that row's proposal.
    #[test]
    fn a_change_the_model_wrote_joins_the_card() {
        let settings = AppSettings::default();
        let mut card = card(&settings, &Map::new(), &Map::new(), &[]).unwrap();
        let mut cards = vec![
            Card {
                items: vec![Item {
                    key: "paste_last_hotkey".into(),
                    to: Value::String("control+shift+v".into()),
                    included: true,
                    ..Item::default()
                }],
                ..Card::default()
            },
            Card {
                items: vec![Item {
                    key: "sound_effects".into(),
                    to: Value::Bool(false),
                    included: true,
                    ..Item::default()
                }],
                ..Card::default()
            },
        ];
        take(&mut card, &mut cards);
        assert_eq!(cards.len(), 1);
        let row = card
            .items
            .iter()
            .find(|item| item.key == "paste_last_hotkey")
            .unwrap();
        assert!(row.included);
        assert_eq!(row.to, Value::String("control+shift+v".into()));
    }
}
