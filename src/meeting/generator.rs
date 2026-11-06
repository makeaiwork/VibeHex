//! What writes meeting titles, notes, and live chapters: the engine chosen
//! in Meetings › Settings (Codex, OpenRouter, Your server, a built-in model,
//! or OpenCode, as AI cleanup set them up), falling back to dictation's AI
//! cleanup engine and then OpenCode when it fails; else, as before, Apple
//! Intelligence on a Mac that has it and allows it, otherwise the OpenCode
//! model when the user turned that on. Every engine but Apple Intelligence
//! receives the transcript, so each is opt-in on every OS, and writes full
//! notes from the summary prompt; Apple Intelligence writes two sentences.

use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::app_settings::{AppSettings, CleanupServerSettings, DictationMode};
use crate::dictation::processor::Model;
use crate::files::rewrite::Engine;

/// OpenCode's models take far more than any excerpt Hex sends.
const OPENCODE_CONTEXT_TOKENS: usize = 32_000;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct GeneratedNotes {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    /// The model that wrote full notes; empty for a two-sentence summary.
    #[serde(default)]
    pub written_by: String,
    /// Why the chosen engine didn't write them, when another one did.
    #[serde(default)]
    pub fell_back: Option<String>,
}

/// One live chapter pass: what the new lines are about, and whether that is
/// still the current chapter's topic.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct GeneratedChapter {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub same_topic: bool,
}

#[derive(Deserialize)]
pub(crate) struct GeneratedChapterLines {
    #[serde(default)]
    pub(crate) lines: Vec<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationError {
    /// The prompt exceeded the model's context window.
    ContextWindow,
    /// The model refused the prompt or its guardrails blocked it.
    Refused,
    Failed(String),
}

impl std::fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContextWindow => formatter.write_str("the transcript exceeded the model context"),
            Self::Refused => formatter.write_str("the model refused the transcript"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Writer {
    #[cfg(target_os = "macos")]
    AppleIntelligence,
    OpenCode,
    /// The engine Meetings › Settings chose, with its fallbacks.
    Engine,
}

/// How notes ask for their answer, after the summary prompt.
const NOTES_FORM: &str = "Answer in Markdown. The first line is `# ` and a short, specific title \
for the whole meeting of at most eight words, in the meeting's language, without dates or the \
word meeting. Then the notes.";

/// A whole meeting's notes may take a while on a large model.
const ENGINE_CALL_TIMEOUT: Duration = Duration::from_secs(300);

/// What notes need from settings, as they last applied.
#[derive(Clone, Debug, Default)]
pub struct NotesConfig {
    writer: Option<String>,
    effort: String,
    /// The summary prompt in the speech model's language.
    prompt: String,
    server: CleanupServerSettings,
    server_on: bool,
    mode: DictationMode,
}

impl NotesConfig {
    pub fn from_settings(settings: &AppSettings) -> Self {
        let (language, _) =
            crate::app_settings::prompt_defaults::prompt_language(&settings.transcription);
        let notes = &settings.meetings.notes;
        Self {
            writer: notes
                .writer
                .clone()
                .filter(|writer| !writer.trim().is_empty()),
            effort: notes.effort().to_owned(),
            prompt: notes.prompt(language),
            server: settings.dictation_processing.server.clone(),
            server_on: settings.labs.ai_cleanup_server,
            mode: settings.dictation_processing.default_mode.clone(),
        }
    }

    fn resolve(&self, choice: Option<&str>) -> Engine {
        Engine::resolve(choice, &self.server, self.server_on, &self.mode).with_effort(&self.effort)
    }

    /// The chosen engine: any but OpenCode needs Labs › AI cleanup on.
    fn chosen(&self) -> Option<Engine> {
        let writer = self.writer.as_deref()?;
        (writer == "opencode" || self.server_on).then(|| self.resolve(Some(writer)))
    }

    /// The chosen engine, then dictation's AI cleanup engine, then OpenCode
    /// while it runs, each once.
    fn engines(&self) -> Vec<Engine> {
        let mut engines: Vec<Engine> = self.chosen().into_iter().collect();
        if self.server_on {
            engines.push(self.resolve(None));
        }
        if crate::desktop::opencode_service::running() {
            engines.push(self.resolve(Some("opencode")));
        }
        let mut seen = Vec::new();
        engines.retain(|engine| {
            let name = engine.model_name();
            let new = !seen.contains(&name);
            seen.push(name);
            new
        });
        engines
    }
}

static CONFIG: RwLock<Option<NotesConfig>> = RwLock::new(None);

/// Records what notes need from settings (`AppSettings::apply_runtime`).
pub(crate) fn configure(config: NotesConfig) {
    *CONFIG.write().unwrap_or_else(|error| error.into_inner()) = Some(config);
}

fn config() -> Option<NotesConfig> {
    CONFIG
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// The writer for a meeting's notes now: the one the user turned on, or,
/// when asked for notes directly, Apple Intelligence wherever it is
/// available, since it never leaves the Mac. Looking for OpenCode asks its
/// running service over loopback, so call this off the UI thread.
fn writer() -> Result<Writer, String> {
    if let Some(writer) = chosen() {
        return Ok(writer);
    }
    #[cfg(target_os = "macos")]
    {
        let availability = crate::apple_intelligence::availability();
        if availability.is_available() {
            return Ok(Writer::AppleIntelligence);
        }
        if !crate::app_settings::meeting_notes_with_opencode() {
            return Err(availability.description().to_string());
        }
    }
    Err(if crate::app_settings::meeting_notes_with_opencode() {
        "OpenCode isn't running".into()
    } else {
        "Turn on Summarize with OpenCode to write meeting notes".into()
    })
}

/// The writer the user turned on, when it is available.
fn chosen() -> Option<Writer> {
    if config().is_some_and(|config| config.chosen().is_some()) {
        return Some(Writer::Engine);
    }
    #[cfg(target_os = "macos")]
    if crate::app_settings::meeting_notes_enabled()
        && crate::apple_intelligence::availability().is_available()
    {
        return Some(Writer::AppleIntelligence);
    }
    (crate::app_settings::meeting_notes_with_opencode()
        && crate::desktop::opencode_service::running())
    .then_some(Writer::OpenCode)
}

/// Whether finished meetings get notes, and recording ones titles,
/// without being asked.
pub(crate) fn automatic() -> bool {
    chosen().is_some()
}

/// Whether notes can be written when asked; the reason when they cannot.
pub(crate) fn available() -> Result<(), String> {
    writer().map(|_| ())
}

/// The writer's context window in tokens, or zero when none is available.
pub(crate) fn context_tokens() -> usize {
    match writer() {
        #[cfg(target_os = "macos")]
        Ok(Writer::AppleIntelligence) => crate::apple_intelligence::context_tokens(),
        Ok(Writer::OpenCode) => OPENCODE_CONTEXT_TOKENS,
        Ok(Writer::Engine) => config()
            .and_then(|config| config.chosen())
            .map_or(OPENCODE_CONTEXT_TOKENS, |engine| engine.notes_window()),
        Err(_) => 0,
    }
}

/// Whether the writer writes full notes from the summary prompt rather
/// than two sentences.
pub(crate) fn full_notes() -> bool {
    matches!(writer(), Ok(Writer::OpenCode | Writer::Engine))
}

/// The notes' instructions: the summary prompt and the answer's form for
/// full notes, else `short`.
pub(crate) fn notes_instructions(short: &str) -> String {
    if !full_notes() {
        return short.to_owned();
    }
    let prompt = config().map_or_else(
        || crate::app_settings::meeting_notes::default_prompt("en").to_owned(),
        |config| config.prompt,
    );
    format!("{prompt}\n\n{NOTES_FORM}")
}

/// How much transcript one notes call may carry: a large window takes the
/// whole meeting, Apple Intelligence `short`.
pub(crate) fn excerpt_cap(short: usize) -> usize {
    if full_notes() { 1_000_000 } else { short }
}

/// How long one notes call may take.
pub(crate) fn call_timeout(short: Duration) -> Duration {
    if matches!(writer(), Ok(Writer::Engine)) {
        ENGINE_CALL_TIMEOUT
    } else {
        short
    }
}

/// A meeting title and a short summary.
pub(crate) fn generate_notes(
    instructions: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<GeneratedNotes, GenerationError> {
    match writer().map_err(GenerationError::Failed)? {
        #[cfg(target_os = "macos")]
        Writer::AppleIntelligence => {
            crate::apple_intelligence::generate_notes(instructions, prompt, timeout)
        }
        Writer::OpenCode => opencode::<GeneratedNotes>(
            instructions,
            prompt,
            r#"{"title": "...", "summary": "..."}"#,
            timeout,
        )
        .map(|notes| GeneratedNotes {
            written_by: "OpenCode".into(),
            ..notes
        }),
        Writer::Engine => engine_notes(instructions, prompt, timeout),
    }
}

/// Full notes from the chosen engine, or the first fallback that answers.
fn engine_notes(
    instructions: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<GeneratedNotes, GenerationError> {
    let config = config().ok_or_else(|| GenerationError::Failed("no notes writer".into()))?;
    let deadline = Instant::now() + timeout;
    let cancelled = AtomicBool::new(false);
    let mut failures = Vec::new();
    for engine in config.engines() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match engine.answer(instructions, prompt, left, &cancelled) {
            Ok(text) => {
                let (title, notes) = titled(&text);
                return Ok(GeneratedNotes {
                    title,
                    summary: notes,
                    written_by: engine.model_name(),
                    fell_back: (!failures.is_empty()).then(|| failures.join("; ")),
                });
            }
            Err(error) => {
                tracing::warn!(error = %format!("{error:#}"), model = %engine.model_name(), "notes writer failed");
                failures.push(format!("{}: {}", engine.model_name(), error));
            }
        }
    }
    Err(GenerationError::Failed(if failures.is_empty() {
        "no notes writer is available".into()
    } else {
        failures.join("; ")
    }))
}

/// The `# Title` line and the notes under it; without one, no title.
fn titled(text: &str) -> (String, String) {
    let text = text.trim();
    let body = text
        .strip_prefix("```markdown")
        .or_else(|| text.strip_prefix("```"))
        .map_or(text, |inner| inner.trim_end_matches("```").trim());
    let mut lines = body.lines();
    match lines.next() {
        Some(first) if first.trim_start().starts_with("# ") => (
            first.trim_start().trim_start_matches('#').trim().to_owned(),
            lines.collect::<Vec<_>>().join("\n").trim().to_owned(),
        ),
        _ => (String::new(), body.to_owned()),
    }
}

/// Only a meeting title.
pub(crate) fn generate_title(
    instructions: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<String, GenerationError> {
    match writer().map_err(GenerationError::Failed)? {
        #[cfg(target_os = "macos")]
        Writer::AppleIntelligence => {
            crate::apple_intelligence::generate_title(instructions, prompt, timeout)
        }
        Writer::OpenCode => {
            opencode::<GeneratedNotes>(instructions, prompt, r#"{"title": "..."}"#, timeout)
                .map(|notes| notes.title)
        }
        Writer::Engine => {
            engine_json::<GeneratedNotes>(instructions, prompt, r#"{"title": "..."}"#, timeout)
                .map(|notes| notes.title)
        }
    }
}

/// One live chapter pass over new transcript lines.
pub(crate) fn generate_chapter(
    instructions: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<GeneratedChapter, GenerationError> {
    match writer().map_err(GenerationError::Failed)? {
        #[cfg(target_os = "macos")]
        Writer::AppleIntelligence => {
            crate::apple_intelligence::generate_chapter(instructions, prompt, timeout)
        }
        Writer::OpenCode => opencode(
            instructions,
            prompt,
            r#"{"title": "...", "summary": "...", "same_topic": true}"#,
            timeout,
        ),
        Writer::Engine => engine_json(
            instructions,
            prompt,
            r#"{"title": "...", "summary": "...", "same_topic": true}"#,
            timeout,
        ),
    }
}

/// The L numbers of the lines about the chapter after a boundary.
pub(crate) fn generate_chapter_lines(
    instructions: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<Vec<i64>, GenerationError> {
    match writer().map_err(GenerationError::Failed)? {
        #[cfg(target_os = "macos")]
        Writer::AppleIntelligence => {
            crate::apple_intelligence::generate_chapter_lines(instructions, prompt, timeout)
        }
        Writer::OpenCode => opencode::<GeneratedChapterLines>(
            instructions,
            prompt,
            r#"{"lines": [12, 13]}"#,
            timeout,
        )
        .map(|generated| generated.lines),
        Writer::Engine => engine_json::<GeneratedChapterLines>(
            instructions,
            prompt,
            r#"{"lines": [12, 13]}"#,
            timeout,
        )
        .map(|generated| generated.lines),
    }
}

/// One JSON answer of `shape` from the chosen engine, or the first
/// fallback that answers.
fn engine_json<T: DeserializeOwned>(
    instructions: &str,
    prompt: &str,
    shape: &str,
    timeout: Duration,
) -> Result<T, GenerationError> {
    let config = config().ok_or_else(|| GenerationError::Failed("no notes writer".into()))?;
    let input = format!(
        "{prompt}\n\nAnswer with only a JSON object shaped like {shape}, without code fences or \
         any other text."
    );
    let deadline = Instant::now() + timeout;
    let cancelled = AtomicBool::new(false);
    let mut last = GenerationError::Failed("no notes writer is available".into());
    for engine in config.engines() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match engine.answer(instructions, &input, left, &cancelled) {
            Ok(text) => match parse_json(&text) {
                Ok(parsed) => return Ok(parsed),
                Err(error) => last = error,
            },
            Err(error) => {
                last = GenerationError::Failed(format!("{}: {error}", engine.model_name()))
            }
        }
    }
    Err(last)
}

/// One OpenCode generation answered as JSON of `shape`.
fn opencode<T: DeserializeOwned>(
    instructions: &str,
    prompt: &str,
    shape: &str,
    timeout: Duration,
) -> Result<T, GenerationError> {
    let request = format!(
        "{instructions}\n\n{prompt}\n\nAnswer with only a JSON object shaped like {shape}, \
         without code fences or any other text."
    );
    let cancelled = AtomicBool::new(false);
    // Labs › Choose meeting summary model: exactly the chosen model, whose
    // failure is the meeting's notes error; otherwise the ranked models.
    let text = match summary_model() {
        Some(model) => {
            crate::dictation::processor::generate_text_with(&request, &model, timeout, &cancelled)
                .map_err(|error| {
                GenerationError::Failed(format!("{}/{}: {error:#}", model.provider, model.id))
            })?
        }
        None => crate::dictation::processor::generate_text(&request, timeout, &cancelled)
            .map_err(|error| GenerationError::Failed(format!("{error:#}")))?,
    };
    parse_json(&text)
}

/// The saved `meetings.opencode_model`, as settings last applied it.
static SUMMARY_MODEL: RwLock<Option<String>> = RwLock::new(None);

/// Records the saved `meetings.opencode_model` (`AppSettings::apply_runtime`).
pub(crate) fn set_summary_model(saved: Option<&str>) {
    *SUMMARY_MODEL
        .write()
        .unwrap_or_else(|error| error.into_inner()) = saved.map(str::to_owned);
}

/// A `provider/model` identifier's two halves, or `None` unless both are
/// there. The model half may itself hold slashes (`openrouter/a/b`).
pub(crate) fn parse_model(saved: &str) -> Option<(&str, &str)> {
    let (provider, model) = saved.trim().split_once('/')?;
    let (provider, model) = (provider.trim(), model.trim());
    (!provider.is_empty() && !model.is_empty()).then_some((provider, model))
}

/// The model meeting notes must use: only with Labs › Choose meeting
/// summary model on and a valid `provider/model` saved.
fn summary_model() -> Option<Model> {
    if !crate::labs::meeting_summary_model() {
        return None;
    }
    let saved = SUMMARY_MODEL
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()?;
    let (provider, id) = parse_model(&saved)?;
    Some(Model {
        provider: provider.into(),
        id: id.into(),
        variant: None,
    })
}

/// The JSON object in a model's answer, ignoring code fences or prose
/// around it.
fn parse_json<T: DeserializeOwned>(text: &str) -> Result<T, GenerationError> {
    let start = text.find('{');
    let end = text.rfind('}');
    let object = match (start, end) {
        (Some(start), Some(end)) if start < end => &text[start..=end],
        _ => {
            return Err(GenerationError::Failed(
                "the model answered without JSON".into(),
            ));
        }
    };
    serde_json::from_str(object).map_err(|error| {
        GenerationError::Failed(format!("the model answered invalid JSON: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_read_through_fences_and_prose() {
        let notes: GeneratedNotes = parse_json(
            "Here you go:\n```json\n{\"title\": \"Launch plan\", \"summary\": \"Dates set.\"}\n```",
        )
        .unwrap();
        assert_eq!(notes.title, "Launch plan");
        assert_eq!(notes.summary, "Dates set.");
        let lines: GeneratedChapterLines = parse_json("{\"lines\": [3, 4]}").unwrap();
        assert_eq!(lines.lines, [3, 4]);
        assert!(parse_json::<GeneratedNotes>("no json here").is_err());
    }

    #[test]
    fn full_notes_take_their_title_from_the_first_heading() {
        let (title, notes) = titled("# Launch plan\n\n**Summary:** dates set.\n- Ship v2");
        assert_eq!(title, "Launch plan");
        assert_eq!(notes, "**Summary:** dates set.\n- Ship v2");
        let (title, notes) = titled("```markdown\n# Pricing\nNotes.\n```");
        assert_eq!((title.as_str(), notes.as_str()), ("Pricing", "Notes."));
        let (title, notes) = titled("No heading here.");
        assert_eq!((title.as_str(), notes.as_str()), ("", "No heading here."));
    }

    #[test]
    fn a_summary_model_needs_a_provider_and_a_model() {
        assert_eq!(parse_model(""), None);
        assert_eq!(parse_model("   "), None);
        assert_eq!(parse_model("gpt-5"), None);
        assert_eq!(parse_model("openai/"), None);
        assert_eq!(parse_model("/gpt-5"), None);
        assert_eq!(parse_model(" openai/gpt-5 "), Some(("openai", "gpt-5")));
        assert_eq!(
            parse_model("openrouter/anthropic/claude"),
            Some(("openrouter", "anthropic/claude"))
        );
    }

    #[test]
    fn without_its_labs_toggle_no_summary_model_is_used() {
        // Tests never load settings, so the Labs toggle reads off.
        if !crate::labs::meeting_summary_model() {
            set_summary_model(Some("openai/gpt-5"));
            assert_eq!(summary_model(), None);
            set_summary_model(None);
        }
    }
}
