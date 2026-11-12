//! Setup's first words, before any question: one short paragraph of what
//! is set now, written by the app in Assistant's language from the same
//! [`Facts`] a model reads: the speech model and its language, dictation
//! cleanup and where it runs, who writes meeting notes, where Files cleans
//! up, the Labs features on that setup asks about, and how many terms
//! Vocabulary holds.

use crate::assistant::changes::names::engine_with;
use crate::assistant::facts::{Cleanup, Facts, Fallback, FilesCleanup, Kind, Runs};
use crate::assistant::guide;
use crate::desktop::i18n::{tr, tr_fill, tr_window};

/// The most Labs features named before "and N more".
const LABS: usize = 4;

/// What is set now, as one paragraph.
pub fn now(facts: &Facts) -> String {
    let mut parts = vec![speech(facts)];
    parts.push(match &facts.cleanup {
        Cleanup::Off => tr("No AI cleanup of dictation").to_owned(),
        Cleanup::On(runs) => tr_fill("Dictation cleanup: {}", &engine(runs)),
    });
    if facts.notes.meetings {
        parts.push(tr_fill("Meeting notes: {}", &notes(facts)));
    } else {
        parts.push(tr("Meetings are off").to_owned());
    }
    match &facts.files {
        FilesCleanup::Off => {}
        FilesCleanup::AsDictation(_) => {
            parts.push(tr_fill("Files cleanup: {}", tr("as dictation")));
        }
        FilesCleanup::Own(runs) => parts.push(tr_fill("Files cleanup: {}", &engine(runs))),
    }
    if let Some(labs) = labs(facts) {
        parts.push(tr_fill("Labs on: {}", &labs));
    }
    parts.push(match facts.vocabulary {
        0 => tr("Vocabulary is empty").to_owned(),
        1 => tr("1 term in Vocabulary").to_owned(),
        terms => tr_fill("{} terms in Vocabulary", &terms.to_string()),
    });
    format!("**{}** {}.", tr("Now:"), parts.join(". "))
}

/// `Orukeet, Russian`.
fn speech(facts: &Facts) -> String {
    let language = match facts.speech.language.as_str() {
        "ru" => tr("Russian"),
        "en" => tr("English"),
        "" | "auto" => tr("any language"),
        other => other,
    };
    format!("{}, {language}", facts.speech.model.name)
}

/// An engine by the name the window gives it, with its model as the
/// person named it: `Codex (GPT-6 Luna)`, `Built in (Qwen3 4B)`.
fn engine(runs: &Runs) -> String {
    let name = match runs.kind {
        Kind::BuiltIn | Kind::Server => tr(runs.kind.name()),
        kind => kind.name(),
    };
    engine_with(name, &runs.model)
}

/// Who writes meeting notes: the writer in use, else what Summaries names.
fn notes(facts: &Facts) -> String {
    match (&facts.notes.writer, facts.notes.fallback) {
        (Some(writer), _) => engine(writer),
        (None, Fallback::AppleIntelligence) => "Apple Intelligence".into(),
        (None, Fallback::OpenCode) => "OpenCode".into(),
        (None, Fallback::Off) => tr("none").into(),
    }
}

/// The Labs features on that setup asks about, or Russian vibe coding:
/// `Russian vibe coding, Vocabulary quick add and 3 more`.
fn labs(facts: &Facts) -> Option<String> {
    let mut on: Vec<&str> = guide::articles()
        .iter()
        .filter(|article| article.is_lab())
        .filter(|article| {
            let Some(key) = article.keys.first() else {
                return false;
            };
            (!article.uses.is_empty() || key == "labs.russian_vibe_coding")
                && key.starts_with("labs.")
                && facts.labs_on.contains(&article.title)
        })
        .map(|article| tr_window(&article.title))
        .collect();
    // Russian vibe coding first: it says the most.
    if let Some(at) = on
        .iter()
        .position(|title| *title == tr_window("Russian vibe coding"))
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
