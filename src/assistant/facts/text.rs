//! The one formatter of [`Facts`]: what a model reads of this install
//! ([`told`]: the `<state>` block and the lines beside it), and what a turn
//! keeps of it for the person to open ([`knew`]). A fact that couldn't be
//! told reads "unknown", never "none" or "no".
//!
//! The block goes only while the person lets their settings be read
//! (`assistant.send_settings`); what this computer can't use goes either
//! way, as before, since it names nothing of theirs. Whether an OpenRouter
//! key is saved and which local servers are installed are never written.

use super::{Cleanup, Fact, Facts, FilesCleanup};
use crate::transcription::languages::language_name;
use crate::transcription::models::AUTO_LANGUAGE;

const UNKNOWN: &str = "unknown";

fn yes_no(fact: &Fact<bool>) -> &'static str {
    match fact {
        Fact::Known(true) => "yes",
        Fact::Known(false) => "no",
        Fact::Unknown => UNKNOWN,
    }
}

/// The speech models downloaded, by the names the window shows, each with
/// its id, the one in use marked: `Orukeet (orukeet, in use), Parakeet v3
/// (parakeet_v3)`.
fn downloaded(facts: &Facts) -> String {
    let Some(models) = facts.speech.downloaded.known() else {
        return UNKNOWN.into();
    };
    if models.is_empty() {
        return "none".into();
    }
    models
        .iter()
        .map(|model| match model.id == facts.speech.model.id {
            true => format!("{} ({}, in use)", model.name, model.id),
            false => format!("{} ({})", model.name, model.id),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `Russian (ru)`; `any (auto)` when none is set.
fn language(code: &str) -> String {
    match code.trim() {
        "" | AUTO_LANGUAGE => "any (auto)".into(),
        code => format!("{} ({code})", language_name(code)),
    }
}

/// Who writes meeting notes, in one sentence, so `meetings.summaries` (a
/// fallback) never reads as the writer.
fn notes(facts: &Facts) -> String {
    let fallback = facts.notes.fallback.name();
    match &facts.notes.writer {
        Some(writer) => format!(
            "Meeting notes: written by {} (meetings.notes.writer); the meetings.summaries \
             fallback ({fallback}) is only for when no writer is chosen, and is not used now.",
            writer.name()
        ),
        None => format!(
            "Meeting notes: no writer chosen in meetings.notes.writer, so meetings.summaries \
             decides: {fallback}."
        ),
    }
}

fn screens(facts: &Facts) -> String {
    let Some(screens) = &facts.state.hardware.screens else {
        return UNKNOWN.into();
    };
    let mut kinds = Vec::new();
    if screens.built_in {
        kinds.push("built-in".to_owned());
    }
    if screens.external > 0 {
        kinds.push(format!("{} external", screens.external));
    }
    if kinds.is_empty() {
        kinds.push("none".to_owned());
    }
    let notch = if screens.notch {
        "with a notch"
    } else {
        "no notch"
    };
    format!("{}, {notch}", kinds.join(" and "))
}

fn pointing(facts: &Facts) -> String {
    let Some(pointers) = &facts.state.hardware.pointers else {
        return UNKNOWN.into();
    };
    if pointers.is_empty() {
        return "none found".into();
    }
    pointers
        .iter()
        .map(|pointer| {
            let buttons = if pointer.extra_buttons {
                "may have extra buttons"
            } else {
                "no extra buttons"
            };
            format!("{}, {buttons}", pointer.name)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn listed(items: &[String]) -> String {
    match items.is_empty() {
        true => "none".into(),
        false => items.join(", "),
    }
}

/// The facts a model reads while settings are sent, one a line.
fn lines(facts: &Facts) -> Vec<String> {
    let shortcuts: Vec<String> = facts
        .shortcuts
        .iter()
        .map(|shortcut| format!("{} {}", shortcut.name, shortcut.keys))
        .collect();
    vec![
        format!(
            "OS: {}",
            match facts.state.os {
                "" => UNKNOWN,
                os => os,
            }
        ),
        format!(
            "Speech model in use: {} ({}); recognition language: {}",
            facts.speech.model.name,
            facts.speech.model.id,
            language(&facts.speech.language)
        ),
        format!("Speech models downloaded: {}", downloaded(facts)),
        match &facts.cleanup {
            Cleanup::Off => "Dictation cleanup: off (Clean up with AI is off)".into(),
            Cleanup::On(runs) => format!("Dictation cleanup: {}", runs.name()),
        },
        notes(facts),
        match &facts.files {
            FilesCleanup::Off => "Files cleanup: Labs › Files is off".into(),
            FilesCleanup::AsDictation(runs) => {
                format!("Files cleanup: as dictation ({})", runs.name())
            }
            FilesCleanup::Own(runs) => format!("Files cleanup: {}", runs.name()),
        },
        format!("Assistant runs on: {}", facts.assistant.name()),
        format!("Codex signed in: {}", yes_no(&facts.state.codex_signed_in)),
        format!(
            "OpenCode running: {}",
            yes_no(&facts.state.opencode_running)
        ),
        format!("Labs on: {}", listed(&facts.labs_on)),
        format!(
            "Shortcuts set: {}",
            match shortcuts.is_empty() {
                true => "none".into(),
                false => shortcuts.join("; "),
            }
        ),
        match facts.vocabulary {
            1 => "Vocabulary: 1 term".into(),
            terms => format!("Vocabulary: {terms} terms"),
        },
        format!(
            "Computer model: {}",
            facts.state.hardware.model.as_deref().unwrap_or(UNKNOWN)
        ),
        format!("Screens: {}", screens(facts)),
        format!("Pointing devices: {}", pointing(facts)),
    ]
}

/// The Labs features this computer can't use, each with what it needs:
/// `Notch indicator (needs a screen with a notch)`; empty while the
/// hardware is unknown, or fits them all.
fn unfit_names(facts: &Facts) -> String {
    facts
        .unfit
        .iter()
        .map(|unfit| {
            let needs: Vec<&str> = unfit.needs.iter().map(|need| need.words()).collect();
            format!("{} (needs {})", unfit.title, needs.join(" and "))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a model is told of them, so an answer never suggests one and,
/// asked about one, says what it needs.
fn unfit(facts: &Facts) -> String {
    match unfit_names(facts).as_str() {
        "" => String::new(),
        names => format!(
            "Not on this Mac: {names}. Don't suggest them or offer their cards; asked about one \
             directly, say what it needs and that this Mac doesn't have it.\n"
        ),
    }
}

/// What a model is told while the person's settings aren't sent.
const NOT_SENT: &str = "You don't know the person's current values, cleanup instructions, or \
    Vocabulary; the app shows each value on its card. When an answer needs them, say in one \
    sentence that **Assistant › Settings › Your current settings** lets you read them, with no \
    card.\n";

/// What a model reads of the facts: what this computer can't use, then the
/// `<state>` block, or, with the settings not sent, that it doesn't know
/// them.
pub fn told(facts: &Facts) -> String {
    let mut text = unfit(facts);
    if facts.consent.send_settings {
        text.push_str("<state>\n");
        for line in lines(facts) {
            text.push_str(&line);
            text.push('\n');
        }
        text.push_str("</state>\n");
    } else {
        text.push_str(NOT_SENT);
    }
    text
}

/// What else of the person's went with a question, beside the facts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sent {
    /// Guide articles.
    pub articles: usize,
    /// Their cleanup instructions.
    pub instructions: bool,
    /// Vocabulary entries the question names, of how many there are.
    pub vocabulary: Option<(usize, usize)>,
    /// Excerpts of their recordings.
    pub recordings: usize,
}

/// What a turn keeps of what its model was told, for the person to open
/// under the answer's steps: the facts as written for the model, without
/// the tags, and what else went along, named but never repeated (their
/// instructions, Vocabulary entries, and recordings stay out of it).
pub fn knew(facts: &Facts, sent: &Sent) -> String {
    let mut text = String::new();
    if facts.consent.send_settings {
        for line in lines(facts) {
            text.push_str(&line);
            text.push('\n');
        }
    } else {
        text.push_str("Your current settings weren't sent (Assistant › Settings).\n");
    }
    match unfit_names(facts).as_str() {
        "" => {}
        names => text.push_str(&format!("Not on this Mac: {names}\n")),
    }
    let mut also = Vec::new();
    match sent.articles {
        0 => {}
        1 => also.push("1 guide article".to_owned()),
        articles => also.push(format!("{articles} guide articles")),
    }
    if facts.consent.send_settings {
        also.push("each changeable setting's value now".to_owned());
    }
    if sent.instructions {
        also.push("your cleanup instructions".to_owned());
    }
    if let Some((named, total)) = sent.vocabulary {
        also.push(format!("{named} of {total} Vocabulary entries"));
    }
    match sent.recordings {
        0 => {}
        1 => also.push("1 excerpt of your recordings".to_owned()),
        excerpts => also.push(format!("{excerpts} excerpts of your recordings")),
    }
    if !also.is_empty() {
        text.push_str(&format!("Also sent: {}.\n", also.join("; ")));
    }
    text
}

/// The record into the debug log, as the turn keeps it: the facts and what
/// else went along by name, nothing of the person's text.
pub fn log(conversation: u32, knew: &str) {
    if !knew.is_empty() {
        tracing::debug!(conversation, "Assistant knew:\n{knew}");
    }
}
