//! What a setting is called in the window, where it lives, and how its
//! values read: a card says "Labs › Smart spacing, Off → On", never
//! `labs.smart_spacing false → true`. Labs features take their guide
//! article's title, which is their Labs row's title; everything else comes
//! from [`WHERE`]. The words are English, as the window's own, and a card
//! shows them through `tr`.
//!
//! Reading a model's value goes the other way: `"OpenCode"`, `"on"`,
//! `"True"`, `"0.3"` become what the key stores.

use serde_json::Value;

use super::parts::{self, Part};
use crate::app_settings::AppSettings;
use crate::control::settings;

/// The pane a setting lives in, where Show in … goes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Home {
    General,
    Output,
    History,
    Meetings,
    /// A Labs feature's row, by its `hex labs` name.
    Labs(&'static str),
    Annotate,
    Files,
}

/// Where a setting is, and what it is called.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    /// `General › Sounds`, levels in English.
    pub path: &'static str,
    pub name: String,
    pub home: Home,
}

/// `(key, path, name, home)` for every setting that isn't a Labs switch.
/// A key that ends in `.` covers the keys under it.
pub const WHERE: &[(&str, &str, &str, Home)] = &[
    ("sound_effects", "General › Sounds", "Sounds", Home::General),
    (
        "sound_effect_volume",
        "General › Sounds",
        "Sound volume",
        Home::General,
    ),
    (
        "sound_theme",
        "General › Sounds",
        "Sound set",
        Home::General,
    ),
    (
        "release_microphone_while_idle",
        "General › Microphone",
        "Turn off the mic when not dictating",
        Home::General,
    ),
    (
        "avoid_bluetooth_microphone",
        "General › Microphone",
        "Avoid Bluetooth microphones",
        Home::General,
    ),
    (
        "recording_audio_behavior",
        "General › Dictation",
        "While dictating",
        Home::General,
    ),
    (
        "recording_duck_volume",
        "General › Dictation",
        "Volume while recording",
        Home::General,
    ),
    (
        "pause_browser_media",
        "General › Dictation",
        "Pause browser media",
        Home::General,
    ),
    (
        "shortcut_mode",
        "General › Shortcut",
        "Shortcut mode",
        Home::General,
    ),
    (
        "double_tap_lock",
        "General › Shortcut",
        "Double-tap",
        Home::General,
    ),
    (
        "double_tap_only",
        "General › Shortcut",
        "Double-tap only",
        Home::General,
    ),
    (
        "dictation_hotkey",
        "General › Shortcut",
        "Shortcut",
        Home::General,
    ),
    (
        "paste_last_hotkey",
        "General › Shortcuts",
        "Paste last dictation",
        Home::General,
    ),
    (
        "cancel_hotkey",
        "General › Shortcuts",
        "Esc to cancel",
        Home::General,
    ),
    (
        "raw_dictation_hotkey",
        "General › Shortcuts",
        "Dictate without processing",
        Home::General,
    ),
    (
        "second_dictation_hotkey",
        "General › Shortcuts",
        "Second dictation shortcut",
        Home::General,
    ),
    (
        "vocabulary_add_hotkey",
        "General › Shortcuts",
        "Add selection to vocabulary",
        Home::General,
    ),
    (
        "stop_hotkey",
        "General › Shortcuts",
        "Stop shortcut",
        Home::General,
    ),
    (
        "typed_text_hotkey",
        "General › Shortcuts",
        "Transform typed text",
        Home::General,
    ),
    (
        "terminal_paste_hotkey",
        "General › Shortcuts",
        "Terminal paste shortcut",
        Home::General,
    ),
    (
        "screenshots.hotkey",
        "Labs › Screenshots",
        "Shortcut",
        Home::Labs("screenshots"),
    ),
    (
        "screenshots.record_hotkey",
        "Labs › Screenshots",
        "Record shortcut",
        Home::Labs("screenshots"),
    ),
    ("show_dock_icon", "General", "Show Dock icon", Home::General),
    (
        "transcription.model",
        "General › Dictation",
        "Speech model",
        Home::General,
    ),
    (
        "transcription.language",
        "General › Dictation",
        "Language",
        Home::General,
    ),
    (
        "vocabulary_boost",
        "General › Vocabulary",
        "Vocabulary",
        Home::General,
    ),
    (
        parts::PROMPT,
        "Output › AI cleanup",
        "Instructions",
        Home::Output,
    ),
    (
        parts::CLEANUP,
        "Output › AI cleanup",
        "Clean up with AI",
        Home::Output,
    ),
    (super::vocab::KEY, "General", "Vocabulary", Home::General),
    (
        super::vocab_edit::KEY,
        "General",
        "Vocabulary",
        Home::General,
    ),
    (
        "dictation_processing.server.runs_on",
        "Output › AI cleanup",
        "Runs on",
        Home::Output,
    ),
    (
        "dictation_processing.server.url",
        "Output › AI cleanup › Your server",
        "URL",
        Home::Output,
    ),
    (
        "dictation_processing.server.openrouter_model",
        "Output › AI cleanup › OpenRouter",
        "Model",
        Home::Output,
    ),
    (
        "dictation_processing.server.custom_model",
        "Output › AI cleanup › Your server",
        "Model",
        Home::Output,
    ),
    (
        "dictation_processing.server.local_model",
        "Output › AI cleanup › Built in",
        "Model",
        Home::Output,
    ),
    (
        "dictation_processing.server.codex_model",
        "Output › AI cleanup › Codex",
        "Model",
        Home::Output,
    ),
    (
        "dictation_processing.server.codex_effort",
        "Output › AI cleanup › Codex",
        "Reasoning",
        Home::Output,
    ),
    (
        "dictation_processing.server.codex_wait_seconds",
        "Output › AI cleanup › Codex",
        "Wait for Codex",
        Home::Output,
    ),
    (
        "history_retention",
        "History",
        "History retention",
        Home::History,
    ),
    (
        "meetings.enabled",
        "Labs",
        "Meetings",
        Home::Labs("meetings"),
    ),
    (
        "meetings.summaries",
        "Meetings › Settings",
        "Summaries",
        Home::Meetings,
    ),
    (
        "meetings.opencode_model",
        "Meetings › Settings",
        "OpenCode model",
        Home::Meetings,
    ),
    (
        "meetings.show_recording_indicator",
        "Meetings › Settings",
        "Show recording indicator",
        Home::Meetings,
    ),
    (
        "meetings.recording_indicator_corner",
        "Meetings › Settings",
        "Indicator corner",
        Home::Meetings,
    ),
    (
        "meetings.retention",
        "Meetings › Settings",
        "Keep meetings",
        Home::Meetings,
    ),
    (
        "meetings.audio_retention",
        "Meetings › Settings",
        "Keep audio",
        Home::Meetings,
    ),
    (
        "meetings.calendar.remind",
        "Meetings › Calendar",
        "Remind me before meetings",
        Home::Meetings,
    ),
    (
        "meetings.calendar.lead_minutes",
        "Meetings › Calendar",
        "Remind me",
        Home::Meetings,
    ),
    (
        "meetings.notes.writer",
        "Meetings › Settings",
        "Written by",
        Home::Meetings,
    ),
    (
        "meetings.notes.effort",
        "Meetings › Settings",
        "Reasoning",
        Home::Meetings,
    ),
    (
        "meetings.questions.enabled",
        "Meetings › Settings",
        "Ask about a meeting",
        Home::Meetings,
    ),
    (
        "meetings.questions.engine",
        "Meetings › Settings",
        "Answers run on",
        Home::Meetings,
    ),
    (
        "meetings.questions.effort",
        "Meetings › Settings",
        "Reasoning",
        Home::Meetings,
    ),
    (
        "experimental.hex_microphone",
        "Labs",
        "Voice Filter",
        Home::Labs("voice-filter"),
    ),
    (
        "experimental.",
        "Labs › Voice Filter",
        "Voice Filter",
        Home::Labs("voice-filter"),
    ),
    (
        "enable_on_launch",
        "General",
        "Enable on launch",
        Home::General,
    ),
    (
        "paste_with_shift",
        "General",
        "Paste with Shift",
        Home::General,
    ),
    (
        "gnome_keyboard_shortcuts",
        "General",
        "GNOME keyboard shortcuts",
        Home::General,
    ),
    (
        "share_dictation_shortcut",
        "General",
        "Desktop shortcuts",
        Home::General,
    ),
    (
        "ui_language",
        "General",
        "Interface language",
        Home::General,
    ),
    (
        "transcription_device",
        "General › Dictation",
        "Speech engine",
        Home::General,
    ),
    (
        "indicator_position",
        "General",
        "Dictation HUD",
        Home::General,
    ),
    (
        "keyboard_model_switching.enabled",
        "Labs",
        "Switch model with keyboard language",
        Home::Labs("keyboard-model-switching"),
    ),
    ("capture.", "Annotate", "Annotate", Home::Annotate),
    (
        "files.cleanup_engine",
        "Files › Settings › Cleanup",
        "Runs on",
        Home::Files,
    ),
    (
        "files.transcription",
        "Files › Settings › Transcription",
        "Speech model",
        Home::Files,
    ),
    (
        "files.use_vocabulary",
        "Files › Settings › Transcription",
        "Use vocabulary",
        Home::Files,
    ),
    (
        "files.show_timestamps",
        "Files › Settings › Transcription",
        "Show timestamps",
        Home::Files,
    ),
    (
        "files.cleanup_after",
        "Files › Settings › Cleanup",
        "Clean up after transcription",
        Home::Files,
    ),
    (
        "files.use_default_prompt",
        "Files › Settings › Cleanup",
        "Use the Default mode's prompt",
        Home::Files,
    ),
    (
        "files.cleanup_effort",
        "Files › Settings › Cleanup",
        "Reasoning",
        Home::Files,
    ),
    (
        "files.request_engine",
        "Files › Settings › Requests",
        "Runs on",
        Home::Files,
    ),
    (
        "files.request_effort",
        "Files › Settings › Requests",
        "Reasoning",
        Home::Files,
    ),
    ("files.", "Labs › Files", "Files", Home::Files),
];

/// Where `key` lives and what it is called, or `None` for a key no card
/// names.
pub fn place(key: &str, settings: &AppSettings) -> Option<Place> {
    if let Some(Part::Provider { label, .. }) = parts::part(key, settings) {
        return Some(Place {
            path: "Meetings › Detected calls",
            name: label,
            home: Home::Meetings,
        });
    }
    if let Some(&(_, path, name, home)) = WHERE.iter().find(|(known, ..)| *known == key) {
        return Some(Place {
            path,
            name: name.to_owned(),
            home,
        });
    }
    if key.starts_with("labs.")
        && let Some(feature) = crate::labs::Feature::for_settings_key(key)
    {
        let name = lab_title(feature.settings_key()).unwrap_or(feature.name());
        // Unload when idle's minutes, Caps Lock's light: a row of a lab.
        if feature.settings_key() != key {
            let what = match key {
                "labs.unload_idle_minutes" => "Minutes",
                "labs.caps_lock_light" => "Light",
                _ => name,
            };
            return Some(Place {
                path: "Labs",
                name: format!("{name} › {what}"),
                home: Home::Labs(feature.name()),
            });
        }
        return Some(Place {
            path: "Labs",
            name: name.to_owned(),
            home: Home::Labs(feature.name()),
        });
    }
    let (_, path, name, home) = WHERE
        .iter()
        .find(|(known, ..)| known.ends_with('.') && key.starts_with(known))?;
    Some(Place {
        path,
        name: (*name).to_owned(),
        home: *home,
    })
}

/// A Labs feature's title, as its guide article and its row give it.
fn lab_title(settings_key: &str) -> Option<&'static str> {
    crate::assistant::guide::articles()
        .iter()
        .find(|article| article.is_lab() && article.keys.iter().any(|key| key == settings_key))
        .map(|article| article.title.as_str())
}

/// The words a card shows for values, which the pane translates (brand
/// names aside): Assistant's own, so every language has them.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    const BRANDS: [&str; 6] = [
        "OpenCode",
        "OpenRouter",
        "Codex",
        "Apple Intelligence",
        "CPU",
        "GPU",
    ];
    ["On", "Off", "Not set", "Empty", "Built in"]
        .into_iter()
        .chain(LABELS.iter().map(|(_, label)| *label))
        .filter(|label| !BRANDS.contains(label))
        .collect()
}

/// Choice ids and the words the window shows for them.
const LABELS: &[(&str, &str)] = &[
    ("opencode", "OpenCode"),
    ("openrouter", "OpenRouter"),
    ("custom", "Your server"),
    ("local", "Built in"),
    ("codex", "Codex"),
    ("mac", "Apple Intelligence"),
    ("off", "Off"),
    ("ask", "Ask"),
    ("always_record", "Always record"),
    ("never_offer", "Never offer"),
    ("hold", "Hold"),
    ("toggle", "Toggle"),
    ("pause_media", "Pause media"),
    ("mute", "Mute"),
    ("duck", "Lower volume"),
    ("do_nothing", "Do nothing"),
    ("hidden", "Hidden"),
    ("day", "A day"),
    ("week", "A week"),
    ("month", "A month"),
    ("forever", "Forever"),
    ("low", "Low"),
    ("medium", "Medium"),
    ("high", "High"),
    ("auto", "Auto"),
    ("cpu", "CPU"),
    ("gpu", "GPU"),
];

/// How `value` of `key` reads on a card: `On`, `OpenCode`, `Parakeet v3`,
/// `30%`.
pub fn label(key: &str, value: &Value) -> String {
    if key == super::vocab::KEY {
        return super::vocab::label(value);
    }
    if key == super::vocab_edit::KEY {
        return super::vocab_edit::entry(value).unwrap_or_default().term;
    }
    match value {
        Value::Bool(true) => "On".into(),
        Value::Bool(false) => "Off".into(),
        Value::Null if key == "meetings.notes.writer" => label(key, &Value::String(String::new())),
        Value::Null => "Not set".into(),
        Value::Number(number) if key == "recording_duck_volume" || key == "sound_effect_volume" => {
            format!("{:.0}%", number.as_f64().unwrap_or(0.0) * 100.0)
        }
        Value::Number(number) => number.to_string(),
        // Meeting notes' writer, empty: as before Codex could write them.
        Value::String(text) if text.is_empty() && key == "meetings.notes.writer" => {
            if cfg!(target_os = "macos") {
                "Apple Intelligence".into()
            } else {
                "OpenCode".into()
            }
        }
        Value::String(text) if text.is_empty() => "Empty".into(),
        // A shortcut as `hex settings` writes it: `right-option` reads
        // Right Option, `option+shift+v` Option+Shift+V.
        Value::String(text) if key.ends_with("hotkey") => shortcut(text),
        // An engine with its model, as the window names both: `Codex
        // (GPT-6 Luna)`, `Built in (Qwen3 4B)`.
        Value::String(text) if text.starts_with("codex:") => engine_with(
            "Codex",
            &crate::dictation::cleanup_server::codex::model_name(&text[6..]),
        ),
        Value::String(text) if text.starts_with("local:") => engine_with(
            "Built in",
            crate::dictation::cleanup_server::local_models::chosen(&text[6..]).name,
        ),
        Value::String(text) => {
            if let Some(name) = model_name(key, text) {
                return name.to_owned();
            }
            if let Some((_, label)) = LABELS.iter().find(|(id, _)| id == text) {
                return (*label).to_owned();
            }
            text.clone()
        }
        other => other.to_string(),
    }
}

/// `Codex (GPT-6 Luna)`; `Codex` when the model has no name of its own.
fn engine_with(engine: &str, model: &str) -> String {
    match model.trim() {
        "" => engine.to_owned(),
        model if model == engine => engine.to_owned(),
        model => format!("{engine} ({model})"),
    }
}

/// A value an engine key stores, compared as the engine it runs: `codex`
/// alone is Codex on AI cleanup's model, so it equals `codex:<that model>`.
fn engine_id(settings: &AppSettings, value: &Value) -> String {
    let text = value
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let model = settings.dictation_processing.server.codex_model.trim();
    match text.as_str() {
        "codex" if !model.is_empty() => format!("codex:{}", model.to_ascii_lowercase()),
        "codex:" => "codex".into(),
        _ => text,
    }
}

/// Whether a key that stores an engine (`meetings.notes.writer`,
/// `files.cleanup_engine`) names one: those values compare as engines.
pub fn is_engine(key: &str) -> bool {
    key.ends_with(".writer") || key.ends_with("_engine") || key.ends_with(".engine")
}

/// Whether `one` and `other`, as `key` stores them, are the same setting:
/// equal, or the same engine written two ways (`codex` and `codex:<the
/// model AI cleanup uses>`; empty and none).
pub fn same(settings: &AppSettings, key: &str, one: &Value, other: &Value) -> bool {
    if one == other {
        return true;
    }
    is_engine(key)
        && matches!(one, Value::String(_) | Value::Null)
        && matches!(other, Value::String(_) | Value::Null)
        && engine_id(settings, one) == engine_id(settings, other)
}

fn shortcut(text: &str) -> String {
    let word = |word: &str| {
        let mut chars = word.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(chars).collect::<String>()
        })
    };
    text.split('+')
        .map(|part| part.split('-').map(word).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("+")
}

/// A speech or built-in cleanup model's name, by its id.
fn model_name(key: &str, id: &str) -> Option<&'static str> {
    match key {
        "transcription.model" => crate::transcription::models::MODELS
            .iter()
            .find(|model| model.id.as_str() == id)
            .map(|model| model.name),
        "dictation_processing.server.local_model" => {
            crate::dictation::cleanup_server::local_models::find(id).map(|model| model.name)
        }
        _ => None,
    }
}

/// `raw`, as a model wrote it for `key`, in the form `key` stores: a
/// switch's `"on"`, a number in a string, a choice's or a model's name.
/// What isn't recognized is passed on as written, for the save to refuse.
pub fn normalize(settings: &AppSettings, key: &str, raw: &Value) -> Value {
    let found = settings::get(settings, key)
        .ok()
        .and_then(|found| found.into_iter().find(|setting| setting.key == key));
    let Some(setting) = found else {
        return match (parts::part(key, settings), raw) {
            (Some(Part::Provider { .. }), Value::String(text)) => {
                Value::String(choice(text, &["ask", "always_record", "never_offer"]))
            }
            (Some(Part::Cleanup), Value::String(text)) => match switch(text) {
                Some(on) => Value::Bool(on),
                None => raw.clone(),
            },
            _ => raw.clone(),
        };
    };
    let text = match raw {
        Value::String(text) => text.trim(),
        _ => "",
    };
    match setting.kind.as_str() {
        "bool" => switch(text).map_or_else(|| raw.clone(), Value::Bool),
        "number" | "integer" => text
            .trim_end_matches('%')
            .parse::<f64>()
            .ok()
            .map(|number| match text.ends_with('%') {
                true => number / 100.0,
                false => number,
            })
            .and_then(|number| match setting.kind.as_str() {
                "integer" if number.fract() == 0.0 && number >= 0.0 => {
                    Some(Value::from(number as u64))
                }
                "integer" => None,
                _ => serde_json::Number::from_f64(number).map(Value::Number),
            })
            .unwrap_or_else(|| raw.clone()),
        "choice" if !text.is_empty() => {
            let choices: Vec<&str> = setting.choices.iter().map(String::as_str).collect();
            Value::String(choice(text, &choices))
        }
        "text" if key == "transcription.model" && !text.is_empty() => {
            let squash = |text: &str| -> String {
                text.chars()
                    .filter(|c| c.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect()
            };
            crate::transcription::models::MODELS
                .iter()
                .find(|model| model.id.as_str() == text || squash(model.name) == squash(text))
                .map_or_else(
                    || raw.clone(),
                    |model| Value::String(model.id.as_str().into()),
                )
        }
        _ => raw.clone(),
    }
}

/// A switch as a model wrote it: `on`, `True`, `yes`, `0`.
fn switch(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Some(true),
        "off" | "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// The choice `text` names, by id or by its label; else `text`.
fn choice(text: &str, choices: &[&str]) -> String {
    let squash = |text: &str| text.to_ascii_lowercase().replace(['-', ' '], "_");
    let wanted = squash(text);
    choices
        .iter()
        .find(|choice| squash(choice) == wanted)
        .or_else(|| {
            choices.iter().find(|choice| {
                LABELS
                    .iter()
                    .any(|(id, label)| id == *choice && squash(label) == wanted)
            })
        })
        .map_or_else(|| text.to_owned(), |choice| (*choice).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys() -> Vec<String> {
        settings::list(&AppSettings::default())
            .into_iter()
            .map(|setting| setting.key)
            .collect()
    }

    /// Every entry names a key this VibeHex has, or keys under a prefix.
    #[test]
    fn every_where_entry_is_a_real_key() {
        let keys = keys();
        for (key, ..) in WHERE {
            let real = if key.ends_with('.') {
                keys.iter().any(|known| known.starts_with(key))
            } else {
                keys.iter().any(|known| known == key)
                    || [
                        parts::PROMPT,
                        parts::CLEANUP,
                        super::super::vocab::KEY,
                        super::super::vocab_edit::KEY,
                    ]
                    .contains(key)
            };
            // Annotate's keys exist only in capture builds.
            assert!(real || key.starts_with("capture."), "{key} is no setting");
        }
    }

    /// Every key a card may change has a place to name it.
    #[test]
    fn every_changeable_key_has_a_place() {
        let settings = AppSettings::default();
        for key in keys() {
            if super::super::catalog::never(&key) {
                continue;
            }
            assert!(place(&key, &settings).is_some(), "{key} has no place");
        }
    }

    #[test]
    fn a_lab_takes_its_guide_title() {
        let settings = AppSettings::default();
        let place = place("labs.smart_spacing", &settings).unwrap();
        assert_eq!((place.path, place.name.as_str()), ("Labs", "Smart spacing"));
        assert_eq!(place.home, Home::Labs("smart-spacing"));
        let zoom = super::place("meetings.providers.zoom.policy", &settings).unwrap();
        assert_eq!(
            (zoom.path, zoom.name.as_str()),
            ("Meetings › Detected calls", "Zoom")
        );
    }

    #[test]
    fn values_read_as_the_window_writes_them() {
        assert_eq!(label("labs.smart_spacing", &json!(true)), "On");
        assert_eq!(
            label("dictation_processing.server.runs_on", &json!("opencode")),
            "OpenCode"
        );
        assert_eq!(
            label("transcription.model", &json!("gigaam_v3")),
            "GigaAM v3"
        );
        assert_eq!(label("recording_duck_volume", &json!(0.3)), "30%");
    }

    #[test]
    fn a_models_words_become_what_the_key_stores() {
        let settings = AppSettings::default();
        let normal = |key: &str, raw: Value| normalize(&settings, key, &raw);
        assert_eq!(normal("labs.smart_spacing", json!("on")), json!(true));
        assert_eq!(normal("labs.smart_spacing", json!("True")), json!(true));
        assert_eq!(normal("labs.smart_spacing", json!(false)), json!(false));
        assert_eq!(normal("recording_duck_volume", json!("0.3")), json!(0.3));
        assert_eq!(normal("recording_duck_volume", json!("30%")), json!(0.3));
        assert_eq!(
            normal("dictation_processing.server.runs_on", json!("OpenCode")),
            json!("opencode")
        );
        assert_eq!(
            normal("dictation_processing.server.runs_on", json!("Your server")),
            json!("custom")
        );
        assert_eq!(
            normal("transcription.model", json!("GigaAM v3")),
            json!("gigaam_v3")
        );
        assert_eq!(
            normal("meetings.providers.zoom.policy", json!("Always record")),
            json!("always_record")
        );
        assert_eq!(normal("labs.smart_spacing", json!("maybe")), json!("maybe"));
        assert_eq!(normal(parts::CLEANUP, json!("on")), json!(true));
    }
}
