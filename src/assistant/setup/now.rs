//! Setup's first words, before any question: one short paragraph of what
//! is set now, written by the app in the window's language. The speech
//! model and its language, dictation cleanup and where it runs, who
//! writes meeting notes, where Files cleans up, the Labs features on that
//! setup asks about, and how many terms Vocabulary holds.

use serde_json::Value;

use crate::app_settings::{AppSettings, MeetingSummaries};
use crate::assistant::changes::{names, parts};
use crate::assistant::guide;
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::{codex, local_models};
use crate::transcription::models;

/// The most Labs features named before "and N more".
const LABS: usize = 4;

/// What is set now, as one paragraph.
pub fn now(settings: &AppSettings) -> String {
    let mut parts = vec![speech(settings)];
    parts.push(cleanup(settings));
    if settings.meetings.enabled {
        parts.push(tr_fill("Meeting notes: {}", &notes(settings)));
    } else {
        parts.push(tr("Meetings are off").to_owned());
    }
    if settings.labs.files {
        let files = match settings.files.cleanup_engine.as_deref() {
            Some(engine) if !engine.trim().is_empty() => engine_name(engine),
            _ => tr("as dictation").to_owned(),
        };
        parts.push(tr_fill("Files cleanup: {}", &files));
    }
    if let Some(labs) = labs(settings) {
        parts.push(tr_fill("Labs on: {}", &labs));
    }
    let terms = settings.vocabulary.len();
    parts.push(match terms {
        0 => tr("Vocabulary is empty").to_owned(),
        1 => tr("1 term in Vocabulary").to_owned(),
        terms => tr_fill("{} terms in Vocabulary", &terms.to_string()),
    });
    format!("**{}** {}.", tr("Now:"), parts.join(". "))
}

/// `Orukeet, Russian`.
fn speech(settings: &AppSettings) -> String {
    let model = models::definition(settings.transcription.model).name;
    let language = match settings.transcription.language.as_str() {
        "ru" => tr("Russian"),
        "en" => tr("English"),
        "" | "auto" => tr("any language"),
        other => other,
    };
    format!("{model}, {language}")
}

/// Dictation cleanup: off, or where it runs and on which model.
fn cleanup(settings: &AppSettings) -> String {
    if parts::Part::Cleanup.value(settings) != Value::Bool(true) {
        return tr("No AI cleanup of dictation").to_owned();
    }
    let server = &settings.dictation_processing.server;
    let engine = if settings.labs.ai_cleanup_server {
        let runs_on = serde_json::to_value(server.runs_on).unwrap_or(Value::Null);
        let id = runs_on.as_str().unwrap_or_default();
        let name = tr(&names::label(
            "dictation_processing.server.runs_on",
            &runs_on,
        ))
        .to_owned();
        let model = match id {
            "local" => local_models::chosen(&server.local_model).name.to_owned(),
            "codex" => codex::model_name(&server.codex_model),
            "openrouter" => server.openrouter_model.clone(),
            "custom" => server.custom_model.clone(),
            _ => String::new(),
        };
        if model.trim().is_empty() || model == name {
            name
        } else {
            format!("{name} ({model})")
        }
    } else {
        "OpenCode".to_owned()
    };
    tr_fill("Dictation cleanup: {}", &engine)
}

/// An engine as Files and notes write it, by name: `Codex (GPT-6 Luna)`.
fn engine_name(engine: &str) -> String {
    let engine = engine.trim();
    if let Some(model) = engine.strip_prefix("codex:") {
        return format!("Codex ({})", codex::model_name(model));
    }
    match engine {
        "codex" => "Codex".into(),
        "opencode" => "OpenCode".into(),
        "openrouter" => "OpenRouter".into(),
        "custom" => tr("Your server").into(),
        id => format!(
            "{} ({})",
            tr("Built in"),
            local_models::chosen(id.strip_prefix("local:").unwrap_or(id)).name
        ),
    }
}

/// Who writes meeting notes.
fn notes(settings: &AppSettings) -> String {
    match settings.meetings.notes.writer.as_deref() {
        Some(writer) if !writer.trim().is_empty() => engine_name(writer),
        _ => match settings.meetings.summaries {
            MeetingSummaries::Mac => "Apple Intelligence".into(),
            MeetingSummaries::OpenCode => "OpenCode".into(),
            MeetingSummaries::Off => tr("none").into(),
        },
    }
}

/// The Labs features on that setup asks about, or Russian vibe coding:
/// `Russian vibe coding, Vocabulary quick add and 3 more`.
fn labs(settings: &AppSettings) -> Option<String> {
    let on: Vec<&str> = guide::articles()
        .iter()
        .filter(|article| article.is_lab())
        .filter(|article| {
            let Some(key) = article.keys.first() else {
                return false;
            };
            (!article.uses.is_empty() || key == "labs.russian_vibe_coding")
                && key.starts_with("labs.")
                && crate::assistant::changes::current(settings, key) == Some(Value::Bool(true))
        })
        .map(|article| tr(&article.title))
        .collect();
    // Russian vibe coding first: it says the most.
    let mut on = on;
    if let Some(at) = on
        .iter()
        .position(|title| *title == tr("Russian vibe coding"))
    {
        let vibe = on.remove(at);
        on.insert(0, vibe);
    }
    match on.len() {
        0 => None,
        count if count <= LABS => Some(on.join(", ")),
        count => Some(format!(
            "{} {}",
            on[..LABS].join(", "),
            tr_fill("and {} more", &(count - LABS).to_string())
        )),
    }
}
