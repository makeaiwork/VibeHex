//! "Set me up like Denis": the owner's own settings, exported by
//! `scripts/export-preset` (an allow-list: no keys, addresses, folders,
//! devices, calendars, or window state) and shipped in the binary. Its
//! settings become one plan, its shortcuts a second step after it.
//!
//! Every line passes the gate (`assistant::offer`) as the preset's: a line
//! this machine can't take is left out and named under "Not available
//! here"; an engine not ready starts unchecked with where to finish; a
//! value already the same folds into "Already set". Everything else
//! starts checked, but a shorter keep-meetings time, which moves
//! recordings to the Trash.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::plan::area;
use crate::app_settings::{AppSettings, HotkeyBinding};
use crate::assistant::changes::parse::Proposal;
use crate::assistant::changes::{self, Card, CardKind, Item, built_in, names, parts, vocab};
use crate::assistant::facts::State;
use crate::assistant::offer::{Gate, Origin, Why};
use crate::control::settings;

/// The owner's preset, as `scripts/export-preset` wrote it.
const DENIS: &str = include_str!("../../../presets/deniskey.json");

/// Kept for last: shortening it moves meetings older than a month to the
/// Trash, so the gate starts it unchecked.
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

/// Whether the preset is offered at all: exactly when the recognition
/// language saved is Russian (`transcription.language` is `ru`), whatever
/// the speech model: one of many languages, GigaAM, Parakeet. It is the
/// owner's setup for dictating Russian: Russian models, Russian word
/// fixes, Russian cleanup instructions. On Auto, or any other language,
/// there is no row for it, no plan from a typed request, and nothing of
/// it in what the app itself tells a model.
///
/// A model that knows only Russian counts by its language like any
/// other, never by the model alone. The app saves it with Russian every
/// time it is chosen (`ModelDefinition::language_for`; with Auto the
/// selection fails `models::validate`), so only a settings file edited
/// by hand pairs it with Auto, and that file gets no preset.
pub fn offered(settings: &AppSettings) -> bool {
    settings.transcription.language.trim() == "ru"
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

/// The preset's settings, prompt, and terms as one plan, each line
/// through the gate as the preset's: what this machine can't take is
/// named in `missing`, what is the same already folds into "Already set"
/// (a Labs feature off on both sides says nothing), and an engine not
/// ready, a Codex row while signed out, and a shorter keep-meetings time
/// start unchecked.
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
    let gate = Gate::new(settings, state);
    // The language is checked against the speech model the preset brings,
    // which downloads first: Russian fits Orukeet, not the English model a
    // fresh install starts on.
    let with_model = preset
        .settings
        .get("transcription.model")
        .and_then(Value::as_str)
        .and_then(|model| model.parse().ok())
        .map(|model| {
            let mut settings = settings.clone();
            settings.transcription.model = model;
            settings
        });
    let language_gate = with_model
        .as_ref()
        .map(|settings| Gate::new(settings, state));
    let mut planned = Planned::default();
    let mut items = Vec::new();
    let mut terms = Vec::new();
    let mut kept = Vec::new();
    for proposal in lines {
        let key = proposal.key.clone();
        let gate = match (&language_gate, key.as_str()) {
            (Some(language_gate), "transcription.language") => language_gate,
            _ => &gate,
        };
        let resolved = changes::resolve(std::slice::from_ref(&proposal), gate, Origin::Preset);
        for dropped in &resolved.dropped {
            match &dropped.why {
                // The same already: kept, unless it is a feature left off.
                Why::Same => {
                    let wanted = proposal.value.clone().unwrap_or(Value::Null);
                    if wanted != Value::Bool(false) {
                        kept.push(Item {
                            key: key.clone(),
                            from: wanted.clone(),
                            to: wanted,
                            ..Item::default()
                        });
                    }
                }
                Why::Has => {}
                // Turning off what this computer doesn't have (another
                // OS's lab, the Notch indicator without a notch) loses
                // nothing: not worth a word.
                Why::Elsewhere(_) | Why::Hardware if proposal.value == Some(Value::Bool(false)) => {
                }
                _ => planned.missing.push(name(&key, settings)),
            }
        }
        if !resolved.refused.is_empty() && key != vocab::KEY {
            planned.missing.push(name(&key, settings));
        }
        for item in resolved.cards.into_iter().flat_map(|card| card.items) {
            match key == vocab::KEY {
                true => terms.push(item),
                false => items.push(item),
            }
        }
    }
    // Cleanup on this computer is one row, starting at Denis's model while
    // it fits here (`changes::built_in`).
    let model = preset.settings.get(built_in::KEY).and_then(Value::as_str);
    built_in::fold(&mut items, &mut kept, model, &state.machine);
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

/// The preset's shortcuts as the shortcuts card (`changes::shortcuts`):
/// a row per shortcut this machine has and whose feature is in use once
/// the preset is in, each from the person's binding to Denis's. A row
/// already the same is no change, and reads as already set. The owner's
/// Stop shortcut keeps its Return while the lab is off; offering it would
/// show a key that does nothing, and saving it would turn the lab on.
pub fn shortcuts(preset: &Preset, settings: &AppSettings) -> Option<Card> {
    let proposed: Map<String, Value> = preset
        .shortcuts
        .iter()
        .filter_map(|(key, raw)| Some((key.clone(), shortcut(raw)?)))
        .collect();
    changes::shortcuts::card(settings, &proposed, &preset.settings, &[])
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

    /// Russian saved as the recognition language and nothing else, whatever
    /// the model. Auto and every other language get no preset, a model that
    /// knows only Russian included.
    #[test]
    fn the_preset_is_offered_with_russian_as_the_recognition_language_alone() {
        use crate::transcription::models::{self, TranscriptionModelId as Model};
        let cases = [
            (Model::ParakeetV3, "ru", true),
            (Model::Orukeet, "ru", true),
            (Model::WhisperLargeV3Turbo, "ru", true),
            (Model::GigaAmV3, "ru", true),
            // By the language, never by the model alone: a file edited by
            // hand, since the app never saves GigaAM v3 with Auto.
            (Model::GigaAmV3, "auto", false),
            (Model::GigaAmV3, "", false),
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
        // Choosing a Russian-only model saves Russian, from any language.
        for language in ["auto", "en", "ru"] {
            let mut settings = AppSettings::default();
            settings.transcription.model = Model::ParakeetV3;
            settings.transcription.language = language.into();
            let chosen = settings.transcription_for_model(Model::GigaAmV3);
            assert_eq!(chosen.language, "ru", "from {language}");
            settings.transcription = chosen;
            assert!(offered(&settings), "GigaAM v3 chosen from {language}");
        }
        let by_hand = models::TranscriptionSelection {
            model: Model::GigaAmV3,
            language: "auto".into(),
        };
        assert!(models::validate(&by_hand).is_err());
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
            "http://",
            "https://",
            "/users/",
        ] {
            assert!(!text.contains(never), "{never}");
        }
        // Call services only as one listed app's choice, never the list.
        let listed: Vec<String> = crate::app_settings::known_meeting_providers(true)
            .map(|(key, _)| format!("meetings.providers.{key}.policy"))
            .collect();
        for key in preset
            .settings
            .keys()
            .filter(|key| key.contains("providers"))
        {
            assert!(listed.contains(key), "{key}");
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
            let unavailable = Gate::new(&settings, &mac()).here(&item.key).is_err();
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
        // All 90 of the owner's terms, as they are: the rule that keeps an
        // ordinary word out of a plan's terms doesn't judge the preset's
        // (`offer`: `Origin::Preset`), so "New Session", "AskTool" and
        // "Cancel" are rows, checked. His own app is not in the file.
        assert_eq!(terms.len(), 90);
        let named: Vec<&str> = terms.iter().map(|terms| terms[0].term.as_str()).collect();
        for term in ["New Session", "AskTool", "Cancel", "Claude Code"] {
            assert!(named.contains(&term), "{term}");
        }
        assert!(!named.iter().any(|term| term.eq_ignore_ascii_case("monki")));
    }

    /// The Labs switches the owner had turned on by 2026-11-12, when the
    /// preset was written again from his settings, go through the gate
    /// like every other line: a row exactly where this VibeHex and this
    /// computer can take the feature, a name under "Not available here"
    /// where they can't, and on a computer the feature doesn't fit (a
    /// notch, a mouse with side buttons), no row either.
    #[test]
    fn the_switches_turned_on_since_pass_the_same_gate() {
        const SINCE: [&str; 8] = [
            "labs.failure_notices",
            "labs.filler_words",
            "labs.follow_live_transcript",
            "labs.meeting_descriptive_filenames",
            "labs.more_call_apps",
            "labs.movable_meeting_capsules",
            "labs.spoken_symbols",
            "labs.stop_when_call_ends",
        ];
        let preset = denis();
        let settings = AppSettings::default();
        let state = mac();
        let planned = plan(&preset, &settings, &state);
        let card = planned.card.unwrap();
        let gate = Gate::new(&settings, &state);
        for key in SINCE {
            assert_eq!(preset.settings.get(key), Some(&Value::Bool(true)), "{key}");
            let row = card.items.iter().find(|item| item.key == key);
            match gate.here(key) {
                Ok(()) => assert!(row.is_some_and(|row| row.included), "{key}"),
                Err(_) => {
                    assert!(row.is_none(), "{key} is offered where it isn't available");
                    assert!(planned.missing.contains(&name(key, &settings)), "{key}");
                }
            }
        }
        // A switch the preset turns on that this computer can't use is no
        // row, whichever switch it is: a Mac mini has no notch.
        use crate::desktop::hardware::{Hardware, Screens};
        let mini = State {
            hardware: Hardware {
                screens: Some(Screens {
                    built_in: false,
                    external: 1,
                    notch: false,
                }),
                ..Hardware::default()
            },
            ..mac()
        };
        let mut with_notch = preset.clone();
        let notch = "labs.notch_indicator";
        with_notch.settings.insert(notch.into(), Value::Bool(true));
        let planned = plan(&with_notch, &settings, &mini);
        let card = planned.card.unwrap();
        assert!(card.items.iter().all(|item| item.key != notch));
        assert!(planned.missing.contains(&name(notch, &settings)));
        let gate = Gate::new(&settings, &mini);
        for item in card
            .items
            .iter()
            .filter(|item| item.key.starts_with("labs."))
        {
            assert!(gate.here(&item.key).is_ok(), "{}", item.key);
        }
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
        // Every row is a shortcut or the mode; a row already the same is
        // no change.
        for item in &card.items {
            assert!(changes::shortcuts::is_key(&item.key), "{}", item.key);
            assert_eq!(item.included, item.from != item.to, "{}", item.key);
        }
    }

    /// Use these saves what was recorded in place of Denis's: one save,
    /// the recorded key in the row's place, the rest Denis's.
    #[test]
    fn a_key_recorded_replaces_denis_s_in_the_save() {
        let settings = AppSettings::default();
        let mut card = shortcuts(&denis(), &settings).unwrap();
        let paste = card
            .items
            .iter()
            .position(|item| item.key == "paste_last_hotkey")
            .unwrap();
        let recorded = Value::String("control+shift+v".into());
        changes::shortcuts::set(&mut card, paste, recorded.clone());
        let values = changes::values(&card, &settings).unwrap();
        assert_eq!(values.get("paste_last_hotkey"), Some(&recorded));
        let saved = settings::check(&settings, &values, &crate::rune_ui::shortcut_rules())
            .unwrap()
            .settings;
        assert_eq!(
            changes::current(&saved, "paste_last_hotkey"),
            Some(recorded)
        );
        if cfg!(target_os = "macos") {
            assert_eq!(
                changes::current(&saved, "dictation_hotkey"),
                Some(Value::String("right-option".into()))
            );
        }
    }

    /// Zoom and both Telemosts record without asking, in one save, and
    /// Summaries, which only decides while no notes writer is chosen,
    /// isn't planned beside the writer.
    #[test]
    fn the_call_services_always_record_and_summaries_stay_out() {
        let settings = AppSettings::default();
        let card = plan(&denis(), &settings, &mac()).card.unwrap();
        let keys: Vec<&str> = card.items.iter().map(|item| item.key.as_str()).collect();
        assert!(!keys.contains(&"meetings.summaries"), "{keys:?}");
        let values = changes::values(&card, &settings).unwrap();
        let providers = values
            .get("meetings.providers")
            .expect("the policies are planned");
        for app in [
            "us.zoom.xos",
            crate::app_settings::TELEMOST_APP,
            crate::app_settings::TELEMOST_SITE,
        ] {
            let policy = providers
                .as_array()
                .and_then(|all| all.iter().find(|choice| choice["key"] == app))
                .map(|choice| choice["policy"].clone());
            assert_eq!(policy, Some(Value::String("always_record".into())), "{app}");
        }
    }

    /// Every setting the preset carries is one a plan can save, and on a
    /// Mac nothing it carries is named as missing: turning off another
    /// OS's lab, or one this Mac's hardware can't use, loses nothing.
    #[test]
    fn nothing_the_preset_carries_is_missing_on_a_mac() {
        let preset = denis();
        for key in preset.settings.keys() {
            assert!(!changes::catalog::never(key), "{key}");
        }
        if cfg!(target_os = "macos") {
            // `mac()` has no notch and no extra mouse buttons.
            let planned = plan(&preset, &AppSettings::default(), &mac());
            assert!(planned.missing.is_empty(), "{:?}", planned.missing);
        }
    }

    /// A shortcut whose feature the preset leaves off is neither shown nor
    /// saved: the owner's Stop shortcut keeps Return while its lab is off,
    /// and saving it would turn the lab on.
    #[test]
    fn a_shortcut_whose_feature_stays_off_is_left_out() {
        let mut preset = denis();
        assert_eq!(
            preset.settings.get("labs.stop_shortcut"),
            Some(&Value::Bool(false))
        );
        assert!(preset.shortcuts.contains_key("stop_hotkey"));
        let settings = AppSettings::default();
        let rows =
            |preset: &Preset| shortcuts(preset, &settings).map_or(0, |card| card.items.len());
        let off = rows(&preset);
        let saved: Vec<String> = shortcuts(&preset, &settings)
            .into_iter()
            .flat_map(|card| card.items)
            .map(|item| item.key)
            .collect();
        assert!(
            !saved
                .iter()
                .any(|key| key == "stop_hotkey" || key == "labs.stop_shortcut")
        );
        // On in the preset, it is offered with it.
        preset
            .settings
            .insert("labs.stop_shortcut".into(), Value::Bool(true));
        if cfg!(target_os = "macos") {
            assert_eq!(rows(&preset), off + 1);
        }
    }
}
