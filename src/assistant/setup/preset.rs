//! "Set me up like Denis": the owner's own settings, exported by
//! `scripts/export-preset` (an allow-list: no keys, addresses, folders,
//! devices, calendars, or window state) and shipped in the binary. Its
//! settings become one plan, its shortcuts a second step after it.
//!
//! The setup rules hold: a line this machine can't take is left out and
//! named under "Not available here"; an engine not ready starts unchecked
//! with where to finish; a value already the same folds into "Already
//! set". Everything else starts checked, but a shorter keep-meetings time,
//! which moves recordings to the Trash.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::engines;
use super::plan::{area, line_key};
use crate::app_settings::{AppSettings, HotkeyBinding};
use crate::assistant::changes::catalog;
use crate::assistant::changes::parse::Proposal;
use crate::assistant::changes::{self, Card, CardKind, Item, names, parts, vocab};
use crate::assistant::facts::State;
use crate::control::settings;

/// The owner's preset, as `scripts/export-preset` wrote it.
const DENIS: &str = include_str!("../../../presets/deniskey.json");

/// Keys whose value names an engine, maybe with its model (`codex:gpt-…`).
const ENGINES: [&str; 5] = [
    "dictation_processing.server.runs_on",
    "meetings.notes.writer",
    "meetings.questions.engine",
    "files.cleanup_engine",
    "files.request_engine",
];

/// Kept for last and never checked for the person: it moves meetings
/// older than a month to the Trash.
const RETENTION: &str = "meetings.retention";

/// A preset: `hex settings` keys with the values settings.json holds, the
/// shortcuts (bindings as settings.json holds them, and how they work),
/// the Global mode's prompt, and Vocabulary terms.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub settings: Map<String, Value>,
    pub shortcuts: Map<String, Value>,
    pub prompt: String,
    pub vocabulary: Vec<Value>,
}

/// Whether the preset is offered at all: only to someone whose speech
/// recognition is Russian and nothing else (Russian set as the language,
/// or a speech model that knows only Russian), as `assistant::language`
/// resolves it. It is the owner's setup for dictating Russian: Russian
/// models, Russian word fixes, Russian cleanup instructions. On Auto, or
/// any other language, there is no row for it, no plan from a typed
/// request, and nothing of it in what the app itself tells a model.
pub fn offered(settings: &AppSettings) -> bool {
    crate::assistant::language::spoken(settings).as_deref() == Some("ru")
}

/// The owner's preset.
pub fn denis() -> Preset {
    serde_json::from_str(DENIS).unwrap_or_default()
}

/// A preset as a plan, with the names of what this machine can't take.
#[derive(Clone, Debug, Default)]
pub struct Planned {
    pub card: Option<Card>,
    pub missing: Vec<String>,
}

/// The preset's settings, prompt, and terms as one plan.
pub fn plan(preset: &Preset, settings: &AppSettings, state: &State) -> Planned {
    let mut lines: Vec<Proposal> = preset
        .settings
        .iter()
        .map(|(key, value)| line(key, value.clone()))
        .collect();
    if !preset.prompt.trim().is_empty() {
        lines.push(line(parts::PROMPT, Value::String(preset.prompt.clone())));
    }
    lines.extend(
        preset
            .vocabulary
            .iter()
            .map(|term| line(vocab::KEY, term.clone())),
    );
    let mut planned = Planned::default();
    let mut items = Vec::new();
    let mut terms = Vec::new();
    let mut kept = Vec::new();
    for proposal in lines {
        let key = proposal.key.clone();
        if settings::unavailable(&line_key(&key, settings)).is_some() {
            planned.missing.push(name(&key, settings));
            continue;
        }
        let wanted = proposal.value.clone().unwrap_or(Value::Null);
        if key != vocab::KEY && changes::current(settings, &key) == Some(wanted.clone()) {
            // A Labs feature off in both says nothing worth folding.
            if wanted != Value::Bool(false) {
                kept.push(Item {
                    key,
                    from: wanted.clone(),
                    to: wanted,
                    ..Item::default()
                });
            }
            continue;
        }
        let shortcuts = crate::rune_ui::shortcut_rules();
        let (cards, refused) = changes::resolve(&[proposal], settings, &shortcuts);
        if !refused.is_empty() && key != vocab::KEY {
            planned.missing.push(name(&key, settings));
        }
        for item in cards.into_iter().flat_map(|card| card.items) {
            if key == vocab::KEY {
                terms.push(item);
            } else if unsupported(&item, settings) {
                // It needs a Labs feature this OS doesn't have.
                planned.missing.push(name(&key, settings));
            } else {
                items.push(ready(item, settings, state));
            }
        }
    }
    items.sort_by_key(|item| (area(&item.key), item.key == RETENTION));
    let at = items
        .iter()
        .position(|item| area(&item.key) > super::plan::Area::Vocabulary)
        .unwrap_or(items.len());
    items.splice(at..at, vocab::split(terms));
    kept.sort_by_key(|item| area(&item.key));
    planned.missing.dedup();
    planned.card = (!items.is_empty() || !kept.is_empty()).then(|| Card {
        kind: CardKind::Plan,
        items,
        kept,
        ..Card::default()
    });
    planned
}

/// Whether saving `item` alone fails because this OS doesn't have what
/// it needs (a key whose Labs feature is another OS's).
fn unsupported(item: &Item, settings: &AppSettings) -> bool {
    let one = Card {
        items: vec![item.clone()],
        ..Card::default()
    };
    let Ok(values) = changes::values(&one, settings) else {
        return false;
    };
    let shortcuts = crate::rune_ui::shortcut_rules();
    settings::check(settings, &values, &shortcuts).is_err_and(|error| error.code == "unsupported")
}

/// The preset's shortcuts as a plan, each as `hex settings set` takes it;
/// those already the same, or that this machine can't take, left out.
pub fn shortcuts(preset: &Preset, settings: &AppSettings) -> Option<Card> {
    let rules = crate::rune_ui::shortcut_rules();
    let mut items = Vec::new();
    for (key, raw) in &preset.shortcuts {
        let Some(value) = shortcut(raw) else {
            continue;
        };
        if settings::unavailable(key).is_some()
            || changes::current(settings, key) == Some(value.clone())
        {
            continue;
        }
        let (cards, _) = changes::resolve(&[line(key, value)], settings, &rules);
        items.extend(cards.into_iter().flat_map(|card| card.items));
    }
    (!items.is_empty()).then(|| Card {
        kind: CardKind::Plan,
        items,
        ..Card::default()
    })
}

/// The preset's shortcuts as the person reads them: where each is (only
/// when another has the same name), its name, and its binding.
pub fn shortcut_list(preset: &Preset, settings: &AppSettings) -> Vec<(String, String, String)> {
    let rows: Vec<(String, String, String)> = preset
        .shortcuts
        .iter()
        .filter(|(key, _)| settings::unavailable(key).is_none())
        .filter_map(|(key, raw)| {
            let value = shortcut(raw)?;
            let place = names::place(key, settings);
            let path = place.as_ref().map(|place| place.path.to_owned());
            Some((
                path.unwrap_or_default(),
                name(key, settings),
                names::label(key, &value),
            ))
        })
        .collect();
    rows.iter()
        .map(|(path, name, binding)| {
            let twice = rows.iter().filter(|(_, other, _)| other == name).count() > 1;
            let path = if twice { path.clone() } else { String::new() };
            (path, name.clone(), binding.clone())
        })
        .collect()
}

/// A shortcut's value as `hex settings` shows it: a binding as text
/// (`right-option`), a mode or switch as it is.
fn shortcut(raw: &Value) -> Option<Value> {
    match raw {
        Value::Object(_) => {
            let binding: HotkeyBinding = serde_json::from_value(raw.clone()).ok()?;
            Some(Value::String(settings::format_hotkey(&binding)))
        }
        Value::Null => None,
        other => Some(other.clone()),
    }
}

fn line(key: &str, value: Value) -> Proposal {
    Proposal {
        key: key.to_owned(),
        value: Some(value),
        plan: true,
        line: key.to_owned(),
        ..Proposal::default()
    }
}

/// An item as the setup rules start it: an engine not ready, a Codex row
/// while signed out, and a shorter keep-meetings time, unchecked.
fn ready(mut item: Item, settings: &AppSettings, state: &State) -> Item {
    if ENGINES.contains(&item.key.as_str()) {
        let id = item.to.as_str().unwrap_or_default();
        let id = id.split(':').next().unwrap_or_default();
        if let Some(note) = engines::not_ready(id, settings, state) {
            item.included = false;
            item.note = note.to_owned();
        }
    }
    let signed_out = catalog::needs_codex(&item.key, &item.to) && state.codex_signed_in.is(&false);
    if item.key == RETENTION || signed_out {
        item.included = false;
    }
    item
}

/// A key's name as the window shows it.
fn name(key: &str, settings: &AppSettings) -> String {
    names::place(key, settings).map_or_else(|| key.to_owned(), |place| place.name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::facts::Fact;

    fn mac() -> State {
        State {
            os: "macos",
            codex_signed_in: Fact::Known(true),
            opencode_running: Fact::Known(true),
            openrouter_key: Fact::Known(true),
            ..State::default()
        }
    }

    /// Russian recognition and nothing else: Russian set as the language,
    /// or a speech model that knows only Russian. Auto on a model of many
    /// languages, and every other language, get no preset.
    #[test]
    fn the_preset_is_offered_with_russian_recognition_alone() {
        use crate::transcription::models::TranscriptionModelId as Model;
        let cases = [
            (Model::ParakeetV3, "ru", true),
            (Model::Orukeet, "ru", true),
            // GigaAM v3 knows only Russian, whatever the picker says.
            (Model::GigaAmV3, "auto", true),
            (Model::GigaAmV3, "", true),
            (Model::ParakeetV3, "auto", false),
            (Model::ParakeetV3, "", false),
            (Model::ParakeetV3, "en", false),
            (Model::ParakeetV3, "uk", false),
            (Model::ParakeetV3, "de", false),
            (Model::ParakeetUnifiedEnglish, "auto", false),
        ];
        for (model, language, offer) in cases {
            let mut settings = AppSettings::default();
            settings.transcription.model = model;
            settings.transcription.language = language.into();
            assert_eq!(offered(&settings), offer, "{model:?} on {language:?}");
        }
    }

    #[test]
    fn the_shipped_preset_reads_and_holds_nothing_personal() {
        let preset = denis();
        assert!(preset.settings.len() > 20);
        assert!(!preset.prompt.is_empty());
        assert!(!preset.vocabulary.is_empty());
        assert!(preset.shortcuts.contains_key("dictation_hotkey"));
        let text = DENIS.to_lowercase();
        for never in [
            "api_key",
            "apikey",
            "\"dictation_processing.server.url\"",
            "folder",
            "microphone",
            "calendar",
            "sidebar_collapsed",
            "recents",
            "onboarding",
            "migrations",
            "updates.",
            "\"assistant.",
            "applications",
            "browser_hosts",
            "providers",
            "http://",
            "https://",
            "/users/",
        ] {
            assert!(!text.contains(never), "{never}");
        }
        // Shortcuts come in a step of their own.
        for key in preset.settings.keys() {
            assert!(!key.ends_with("hotkey") && key != "shortcut_mode", "{key}");
        }
    }

    /// On a fresh install everything is checked but the keep-meetings
    /// time; the prompt and the terms are in the plan; no shortcut is.
    #[test]
    fn a_fresh_install_gets_the_preset_checked_but_retention() {
        let settings = AppSettings::default();
        let planned = plan(&denis(), &settings, &mac());
        let card = planned.card.unwrap();
        for item in &card.items {
            assert!(!item.key.ends_with("hotkey"), "{}", item.key);
            assert_ne!(item.key, "shortcut_mode");
            let unavailable = settings::unavailable(&line_key(&item.key, &settings)).is_some();
            assert!(!unavailable, "{}", item.key);
            if item.key == RETENTION {
                assert!(!item.included);
            } else {
                assert!(item.included, "{} starts unchecked", item.key);
            }
        }
        let keys: Vec<&str> = card.items.iter().map(|item| item.key.as_str()).collect();
        for key in [
            parts::PROMPT,
            vocab::KEY,
            "labs.russian_vibe_coding",
            "transcription.model",
        ] {
            assert!(keys.contains(&key), "{key}: {keys:?}");
        }
        if settings::unavailable(RETENTION).is_none() {
            assert!(keys.contains(&RETENTION));
        }
        // A row per term.
        let terms: Vec<_> = card
            .items
            .iter()
            .filter(|item| item.key == vocab::KEY)
            .map(|item| vocab::terms(&item.to))
            .collect();
        assert!(terms.iter().all(|terms| terms.len() == 1));
        assert_eq!(terms.len(), denis().vocabulary.len());
    }

    /// What is the same is "Already set"; terms the person has are left
    /// out; an engine not ready starts unchecked with where to finish.
    #[test]
    fn the_owners_own_settings_are_already_set() {
        let mut settings = AppSettings::default();
        let preset = denis();
        let planned = plan(&preset, &settings, &mac());
        let card = planned.card.unwrap();
        let shortcuts = crate::rune_ui::shortcut_rules();
        let values = changes::values(
            &Card {
                items: card
                    .items
                    .iter()
                    // As Apply saves them: the speech model downloads after.
                    .filter(|item| {
                        item.key != vocab::KEY && !item.key.starts_with("transcription.")
                    })
                    .cloned()
                    .collect(),
                ..Card::default()
            },
            &settings,
        )
        .unwrap();
        settings = settings::check(&settings, &values, &shortcuts)
            .unwrap()
            .settings;
        for item in card.items.iter().filter(|item| item.key == vocab::KEY) {
            settings
                .vocabulary
                .extend(vocab::terms(&item.to).into_iter().map(|term| {
                    crate::dictation::vocabulary::VocabularyEntry {
                        term: term.term,
                        sounds_like: term.sounds_like,
                    }
                }));
        }
        let again = plan(&preset, &settings, &mac()).card.unwrap();
        let left: Vec<&str> = again.items.iter().map(|item| item.key.as_str()).collect();
        // Left: the speech model (it downloads after), and the unchecked
        // keep-meetings time.
        assert!(
            left.iter()
                .all(|key| key.starts_with("transcription.") || *key == RETENTION),
            "{left:?}"
        );
        assert!(
            again
                .kept
                .iter()
                .any(|item| item.key == "labs.russian_vibe_coding")
        );
        let no_key = State {
            openrouter_key: Fact::Known(false),
            codex_signed_in: Fact::Known(false),
            ..mac()
        };
        let card = plan(&preset, &AppSettings::default(), &no_key)
            .card
            .unwrap();
        let writer = card
            .items
            .iter()
            .find(|item| item.key == "meetings.notes.writer")
            .unwrap();
        assert!(!writer.included);
    }

    #[test]
    fn the_shortcuts_come_as_text_and_resolve() {
        let preset = denis();
        let card = shortcuts(&preset, &AppSettings::default()).unwrap();
        // Right Option is a Mac's key.
        let dictation = card
            .items
            .iter()
            .find(|item| item.key == "dictation_hotkey");
        if cfg!(target_os = "macos") {
            assert_eq!(dictation.unwrap().to, Value::String("right-option".into()));
        }
        let shortcuts = crate::rune_ui::shortcut_rules();
        let values = changes::values(&card, &AppSettings::default()).unwrap();
        assert!(settings::check(&AppSettings::default(), &values, &shortcuts).is_ok());
        assert!(!shortcut_list(&preset, &AppSettings::default()).is_empty());
    }
}
