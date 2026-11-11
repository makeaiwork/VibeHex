//! Which settings a card may change, how (a plain save, a download, an OS
//! permission after it, a sign-in first), and the list of them a model
//! reads. Never through a card: sign-ins and API keys, folders and paths,
//! microphones, app and site lists, calendar ids, the update channel, and
//! Assistant's own settings (its model, consent, or visibility).
//!
//! What the model reads of the person's settings is only their values for
//! switches, choices, numbers, and known model ids, and only while
//! Assistant › Settings › Your current settings is on: never a path, a
//! key, a list, a host, or a word from their vocabulary.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{names, parts};
use crate::app_settings::AppSettings;
use crate::control::settings;

/// How a card carries a change out.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// A save, as the window's own control makes it.
    #[default]
    Card,
    /// A model to download first; the setting switches when it is ready.
    Download,
    /// A save, then an OS permission only the OS can grant.
    Permission,
    /// Something only the person can do: sign in, paste a key.
    SignIn,
    /// Never from a card.
    Never,
}

/// The OS permission a key needs once it is on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Permission {
    /// Screen & System Audio Recording (Meetings, Lower every app).
    SystemAudio,
    /// Screen Recording (Screenshots, Annotate replay).
    ScreenRecording,
}

/// Keys, and prefixes ending in `.`, that no card changes.
const NEVER: &[&str] = &[
    "assistant.",
    "updates.channel",
    "microphone",
    "transcription_recents",
    "transcription.recognition_hints",
    "dictation_processing.modes",
    "dictation_processing.default_mode",
    "dictation_processing.server.openrouter_api_key",
    "dictation_processing.server.custom_api_key",
    "meetings.calendar.connected",
    "meetings.calendar.excluded_calendars",
    "meetings.notes.prompts",
    "meetings.providers",
    "keyboard_model_switching.mappings",
    "capture.routing",
    "files.transcription",
    "files.quick_prompts",
    "files.last_folder",
    "labs.sidebar_collapsed",
];

/// Keys the model may change though `hex settings` sets them another way.
const DOWNLOADS: [&str; 2] = ["transcription.model", "transcription.language"];

/// Whether no card ever changes `key`.
pub fn never(key: &str) -> bool {
    if parts::PROMPT == key || key.starts_with("meetings.providers.") {
        return false;
    }
    if NEVER
        .iter()
        .any(|never| key == *never || (never.ends_with('.') && key.starts_with(never)))
    {
        return true;
    }
    match settings::kinds().find(|(known, ..)| *known == key) {
        Some((_, kind, _, writable)) => {
            matches!(kind, "path" | "json") || (!writable && !DOWNLOADS.contains(&key))
        }
        // An API key, or no setting at all: `resolve` tells which.
        None => true,
    }
}

/// How a card changes `key` to `value`.
pub fn class(key: &str, value: &Value) -> Class {
    if never(key) {
        return Class::Never;
    }
    match (key, value.as_str()) {
        ("transcription.model" | "transcription.language", _) => Class::Download,
        ("dictation_processing.server.runs_on", Some("local")) => Class::Download,
        _ if permission(key, value).is_some() => Class::Permission,
        _ => Class::Card,
    }
}

/// The permission `key` at `value` needs, if any.
pub fn permission(key: &str, value: &Value) -> Option<Permission> {
    if value != &Value::Bool(true) {
        return None;
    }
    match key {
        "meetings.enabled" | "labs.lower_every_app" => Some(Permission::SystemAudio),
        "labs.screenshots" | "labs.annotate_replay" => Some(Permission::ScreenRecording),
        _ => None,
    }
}

/// Whether `key` at `value` runs on Codex, which needs a ChatGPT sign-in.
pub fn needs_codex(key: &str, value: &Value) -> bool {
    let engine = key == "dictation_processing.server.runs_on"
        || key.ends_with(".writer")
        || key.ends_with("_engine")
        || key.ends_with(".engine");
    engine
        && value
            .as_str()
            .is_some_and(|value| value == "codex" || value.starts_with("codex:"))
}

/// What shortening a retention removes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shorter {
    /// Older meetings move to the Trash.
    Meetings,
    /// Older meetings lose their audio.
    MeetingAudio,
    /// Older dictations leave History.
    History,
    /// Older captures lose their video and audio.
    CaptureMedia,
    /// Older captures are deleted.
    Captures,
}

/// Whether `key` going `from` → `to` keeps less than before, and what it
/// removes. Lengths are compared in days; none and forever keep longest.
pub fn shorter(key: &str, from: &Value, to: &Value) -> Option<Shorter> {
    let what = match key {
        "meetings.retention" => Shorter::Meetings,
        "meetings.audio_retention" => Shorter::MeetingAudio,
        "history_retention" => Shorter::History,
        "capture.media_days" => Shorter::CaptureMedia,
        "capture.bundle_days" => Shorter::Captures,
        _ => return None,
    };
    (days(to) < days(from)).then_some(what)
}

fn days(value: &Value) -> f64 {
    match value {
        Value::Number(number) => number.as_f64().unwrap_or(f64::MAX),
        Value::String(text) => match text.as_str() {
            "off" => 0.0,
            "day" => 1.0,
            "week" => 7.0,
            "month" => 30.0,
            _ => f64::MAX,
        },
        _ => f64::MAX,
    }
}

/// The keys a small model always reads, beside its articles'.
const CORE: [&str; 16] = [
    "transcription.model",
    "transcription.language",
    "labs.ai_cleanup_server",
    "dictation_processing.server.runs_on",
    "meetings.enabled",
    "meetings.notes.writer",
    "dictation_hotkey",
    "shortcut_mode",
    "double_tap_lock",
    "recording_audio_behavior",
    "sound_effects",
    "history_retention",
    "vocabulary_boost",
    "labs.smart_spacing",
    "labs.russian_vibe_coding",
    "ui_language",
];

/// What the snapshot says besides the settings, read by the pane.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub os: &'static str,
    /// Speech model ids downloaded.
    pub installed: Vec<String>,
    pub codex_signed_in: Option<bool>,
    pub opencode_running: Option<bool>,
}

/// How much of the catalog a model reads.
#[derive(Clone, Debug)]
pub struct Reading<'a> {
    /// Every key, or only `keys` and the core ones.
    pub every: bool,
    /// The keys the articles found name.
    pub keys: Vec<&'a str>,
    /// The person's values may go in (`assistant.send_settings`).
    pub send: bool,
    /// The Global mode's instructions go in too (an instructions question).
    pub instructions: bool,
}

/// The settings a card may change, as the prompt carries them: one line
/// each, `key | values | where | now`.
pub fn section(settings: &AppSettings, reading: &Reading, state: &State) -> String {
    let shown = settings::values(settings);
    let wanted = |key: &str| {
        reading.every
            || CORE.contains(&key)
            || reading.keys.iter().any(|wanted| {
                *wanted == key
                    || settings::requires(wanted, &Value::Bool(true))
                        .iter()
                        .any(|(needs, _)| *needs == key)
            })
    };
    let mut out = String::from("<changeable_settings>\nkey | values | where in the window");
    out.push_str(if reading.send { " | now\n" } else { "\n" });
    for (key, kind, choices, _) in settings::kinds() {
        if never(key) || settings::unavailable(key).is_some() || !wanted(key) {
            continue;
        }
        let Some(place) = names::place(key, settings) else {
            continue;
        };
        out.push_str(&format!(
            "{key} | {} | {} › {}",
            values(key, kind, &choices),
            place.path,
            place.name
        ));
        if reading.send {
            let now = shown
                .iter()
                .find(|(known, _)| *known == key)
                .and_then(|(_, value)| sendable(key, kind, value));
            out.push_str(&format!(" | {}", now.unwrap_or_default()));
        }
        out.push('\n');
    }
    if reading.every || reading.keys.iter().any(|key| key.starts_with("meetings.")) {
        out.push_str(
            "meetings.providers.<app>.policy | ask/always_record/never_offer | Meetings › Detected calls › <app>",
        );
        out.push_str(if reading.send { " |\n" } else { "\n" });
    }
    out.push_str(&format!(
        "{} | the Global mode's whole instructions, as text | Output › AI cleanup › Instructions{}\n",
        parts::PROMPT,
        if reading.send { " |" } else { "" },
    ));
    out.push_str("</changeable_settings>\n");
    out.push_str(&format!(
        "Speech model ids: {}\n",
        crate::transcription::models::MODELS
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(
        "Never through a card: sign-ins, API keys, folders, microphones, app lists, the update \
         channel, Assistant's own settings.\n",
    );
    if reading.send {
        out.push_str(&state_line(state));
        if reading.instructions {
            out.push_str(&format!(
                "<global_instructions>\n{}\n</global_instructions>\n",
                settings
                    .dictation_processing
                    .default_mode
                    .post_processing
                    .prompt
                    .trim()
            ));
        }
    } else {
        out.push_str(
            "You don't know the person's current values; the app shows each on its card.\n",
        );
    }
    out
}

/// What a key accepts, in a few words.
fn values(key: &str, kind: &str, choices: &[&str]) -> String {
    match kind {
        "bool" => "on/off".into(),
        "choice" => choices.join("/"),
        "number" | "integer" => match settings::bounds(key) {
            Some((min, max)) => format!("{min} to {max}"),
            None => kind.into(),
        },
        "shortcut" => "a shortcut like option+space, or none".into(),
        _ => match key {
            "transcription.model" => "speech model id (downloads first)".into(),
            "transcription.language" => "language code, or auto".into(),
            "dictation_processing.server.local_model" => "built-in model id".into(),
            _ if key.ends_with("effort") => "low/medium/high".into(),
            _ if key.ends_with("engine") || key.ends_with(".writer") => {
                "codex, codex:<model>, opencode, openrouter, custom, local:<id>".into()
            }
            _ => "text".into(),
        },
    }
}

/// The value a model may read: a switch's, a choice's, a number, or a
/// known model id. Text, paths, lists, and shortcuts never.
fn sendable(key: &str, kind: &str, value: &Value) -> Option<String> {
    match (kind, value) {
        ("bool", Value::Bool(on)) => Some(if *on { "on" } else { "off" }.into()),
        ("choice", Value::String(choice)) => Some(choice.clone()),
        ("choice", Value::Null) => Some("none".into()),
        ("number" | "integer", Value::Number(number)) => Some(number.to_string()),
        ("text", Value::String(id)) if key == "transcription.model" => {
            crate::transcription::models::MODELS
                .iter()
                .any(|model| model.id.as_str() == id)
                .then(|| id.clone())
        }
        ("text", Value::String(id)) if key == "dictation_processing.server.local_model" => {
            crate::dictation::cleanup_server::local_models::find(id).map(|model| model.id.into())
        }
        _ => None,
    }
}

fn state_line(state: &State) -> String {
    let known = |known: Option<bool>| match known {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    };
    format!(
        "<state>OS: {}; speech models downloaded: {}; Codex signed in: {}; OpenCode running: {}</state>\n",
        state.os,
        if state.installed.is_empty() {
            "none".to_owned()
        } else {
            state.installed.join(", ")
        },
        known(state.codex_signed_in),
        known(state.opencode_running),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(every: bool, keys: Vec<&str>, send: bool) -> Reading<'_> {
        Reading {
            every,
            keys,
            send,
            instructions: false,
        }
    }

    /// The keys a built catalog advertises, as written.
    fn advertised(section: &str) -> Vec<String> {
        section
            .lines()
            .skip(2)
            .take_while(|line| !line.starts_with("</"))
            .filter_map(|line| line.split(" | ").next())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn every_registry_key_has_a_class_and_every_never_entry_is_real() {
        let keys: Vec<&str> = settings::kinds().map(|(key, ..)| key).collect();
        for key in &keys {
            let _ = class(key, &Value::Bool(true));
        }
        for never in NEVER {
            let real = keys
                .iter()
                .any(|key| key == never || (never.ends_with('.') && key.starts_with(never)));
            let api_key =
                never.ends_with("_api_key") || *never == "transcription.recognition_hints";
            assert!(real || api_key, "{never} is no setting");
        }
        for never in [
            "assistant.engine",
            "assistant.send_settings",
            "updates.channel",
            "microphone",
        ] {
            assert_eq!(class(never, &Value::Null), Class::Never, "{never}");
        }
        assert_eq!(class("screenshots.folder", &Value::Null), Class::Never);
        assert_eq!(class("labs.smart_spacing", &Value::Bool(true)), Class::Card);
        assert_eq!(
            class("transcription.model", &Value::from("gigaam_v3")),
            Class::Download
        );
        assert_eq!(
            class("dictation_processing.server.runs_on", &Value::from("local")),
            Class::Download
        );
        assert_eq!(
            class("meetings.enabled", &Value::Bool(true)),
            Class::Permission
        );
        assert_eq!(class("meetings.enabled", &Value::Bool(false)), Class::Card);
        assert_eq!(class(parts::PROMPT, &Value::from("x")), Class::Card);
    }

    /// Every line a built prompt advertises is a key `hex settings get`
    /// reads, or a part.
    #[test]
    fn every_advertised_line_resolves() {
        let settings = AppSettings::default();
        let section = section(
            &settings,
            &reading(true, Vec::new(), true),
            &State::default(),
        );
        let keys = advertised(&section);
        assert!(keys.len() > 60, "only {} keys", keys.len());
        for key in keys {
            if key == parts::PROMPT || key == "meetings.providers.<app>.policy" {
                continue;
            }
            assert!(
                settings::get(&settings, &key).is_ok(),
                "{key} doesn't resolve"
            );
            assert!(!never(&key), "{key} is never changed and yet advertised");
        }
    }

    /// The question set's expected changes: each one advertised in the
    /// large catalog, and most in the small one (from its own articles).
    #[test]
    fn the_question_sets_changes_are_advertised() {
        let settings = AppSettings::default();
        let large = advertised(&section(
            &settings,
            &reading(true, Vec::new(), true),
            &State::default(),
        ));
        let shown = |key: &str, keys: &[String]| match key {
            "meetings.providers" => keys
                .iter()
                .any(|known| known.starts_with("meetings.providers.")),
            key => keys.iter().any(|known| known == key),
        };
        let text = include_str!("../../../tests/fixtures/assistant/questions.jsonl");
        let (mut wanted, mut found) = (0, 0);
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let question: Value = serde_json::from_str(line).unwrap();
            let changes = question["changes"].as_array().cloned().unwrap_or_default();
            let asked = question["q"].as_str().unwrap_or_default();
            let articles = crate::assistant::retrieve::search(asked, 3);
            let keys: Vec<&str> = articles
                .iter()
                .flat_map(|article| article.keys.iter().map(String::as_str))
                .collect();
            let small = advertised(&section(
                &settings,
                &reading(false, keys, true),
                &State::default(),
            ));
            for change in changes {
                let key = change["key"].as_str().unwrap();
                if settings::unavailable(key).is_some() {
                    continue;
                }
                assert!(shown(key, &large), "{key} isn't in the large catalog");
                wanted += 1;
                found += usize::from(shown(key, &small));
            }
        }
        assert!(wanted > 25, "only {wanted} changes checked");
        let recall = found as f64 / wanted as f64;
        assert!(
            recall >= 0.9,
            "small catalog recall {recall:.2} ({found} of {wanted})"
        );
    }

    /// What reaches a model of the person's settings: switches, choices,
    /// numbers, and known models. No path, list, host, key, word, or
    /// instruction (unless asked about instructions).
    #[test]
    fn the_snapshot_carries_nothing_personal() {
        let mut settings = AppSettings::default();
        settings.meetings.output_folder = Some("/Users/someone/Secret Meetings".into());
        settings.screenshots.folder = Some("/Users/someone/Shots".into());
        settings.files.last_folder = Some("/Users/someone/Interviews".into());
        settings.dictation_processing.server.url = "http://secret-host.example:1234/v1".into();
        settings
            .dictation_processing
            .default_mode
            .post_processing
            .prompt = "Never mention Project Nightjar.".into();
        settings.dictation_processing.default_mode.applications = vec!["com.secret.app".into()];
        settings
            .vocabulary
            .push(crate::dictation::vocabulary::VocabularyEntry {
                term: "Zyxwvutsr".into(),
                sounds_like: Vec::new(),
            });
        settings.meetings.calendar.excluded_calendars = vec!["CAL-SECRET-ID".into()];
        settings.microphone = Some("Someone's AirPods".into());
        let every = reading(true, Vec::new(), true);
        let section = section(&settings, &every, &State::default());
        for secret in [
            "/Users/someone",
            "secret-host",
            "Nightjar",
            "com.secret.app",
            "Zyxwvutsr",
            "CAL-SECRET-ID",
            "AirPods",
            "_api_key",
        ] {
            assert!(!section.contains(secret), "the snapshot holds {secret}");
        }
        let asked = Reading {
            instructions: true,
            ..every
        };
        assert!(super::section(&settings, &asked, &State::default()).contains("Nightjar"));
        // Off: no values at all.
        let off = super::section(
            &settings,
            &reading(true, Vec::new(), false),
            &State::default(),
        );
        assert!(off.contains("don't know the person's current values"));
        assert!(!off.contains(" | on\n") && !off.contains(" | off\n"));
    }

    #[test]
    fn a_shorter_retention_is_named() {
        assert_eq!(
            shorter(
                "meetings.retention",
                &Value::from("month"),
                &Value::from("week")
            ),
            Some(Shorter::Meetings)
        );
        assert_eq!(
            shorter("meetings.retention", &Value::Null, &Value::from("day")),
            Some(Shorter::Meetings)
        );
        assert_eq!(
            shorter(
                "meetings.retention",
                &Value::from("day"),
                &Value::from("week")
            ),
            None
        );
        assert_eq!(
            shorter(
                "history_retention",
                &Value::from("forever"),
                &Value::from("off")
            ),
            Some(Shorter::History)
        );
        assert_eq!(
            shorter("labs.files", &Value::Bool(true), &Value::Bool(false)),
            None
        );
    }
}
