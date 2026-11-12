//! Where setup's text work runs: round one's two engine questions, read
//! from the settings and turned back into the keys they set. Dictation
//! (`cleanup`) is what the person waits for, so its answer maps to
//! `dictation_processing.server.runs_on` and the Global mode's Clean up
//! with AI switch; meeting notes and files (`notes`) may take longer, so
//! theirs maps to `meetings.notes.writer` and `files.cleanup_engine`.
//!
//! An engine that isn't ready (OpenRouter without a key, your own server
//! without an address, OpenCode not running, Codex signed out) still makes
//! its lines, unchecked, with where to finish: a card never sets a key, an
//! address, or a sign-in.

use serde_json::Value;

use super::plan::Line;
use crate::app_settings::{AppSettings, MeetingSummaries};
use crate::assistant::changes::{current, parts};
use crate::assistant::facts::{self, Cleanup, Fallback, FilesCleanup, Kind, State};
use crate::dictation::cleanup_server::local_models;

const RUNS_ON: &str = "dictation_processing.server.runs_on";
const WRITER: &str = "meetings.notes.writer";
const FILES: &str = "files.cleanup_engine";
const SERVER_LAB: &str = "labs.ai_cleanup_server";

/// An engine as round one ids it.
fn kind_id(kind: Kind) -> &'static str {
    match kind {
        Kind::Codex => "codex",
        Kind::BuiltIn => "local",
        Kind::OpenRouter => "openrouter",
        Kind::Server => "custom",
        Kind::OpenCode => "opencode",
    }
}

/// Where dictation is cleaned up now (`facts::cleanup`), as round one's
/// `cleanup` ids it: `none` while Clean up with AI is off, `opencode`
/// without the lab.
pub fn dictation_now(settings: &AppSettings) -> &'static str {
    match facts::cleanup(settings) {
        Cleanup::Off => "none",
        Cleanup::On(runs) => kind_id(runs.kind),
    }
}

/// Who writes meeting notes now (`facts::notes`), or `""` for none: the
/// writer in use, else Summaries' Apple Intelligence or OpenCode.
fn notes_engine(settings: &AppSettings) -> &'static str {
    let notes = facts::notes(settings);
    match (notes.writer, notes.fallback) {
        (Some(writer), _) => kind_id(writer.kind),
        (None, Fallback::AppleIntelligence) => "apple",
        (None, Fallback::OpenCode) => "opencode",
        (None, Fallback::Off) => "",
    }
}

/// Where Files' cleanup runs now (`facts::files`): its own engine, else
/// dictation's.
fn files_engine(settings: &AppSettings) -> &'static str {
    match facts::files(settings) {
        FilesCleanup::Own(runs) => kind_id(runs.kind),
        _ => dictation_now(settings),
    }
}

/// Round one's `notes` chip from the settings: who writes notes, else
/// where files are cleaned up, else dictation's engine.
pub fn notes_now(settings: &AppSettings, os: &str) -> &'static str {
    let id = match notes_engine(settings) {
        "" => files_engine(settings),
        id => id,
    };
    match id {
        "apple" if os != "macos" => "codex",
        "none" | "" if os == "macos" => "apple",
        "none" | "" => "codex",
        id => id,
    }
}

/// Your own server's name as the address or the Applications folder
/// tells it: LM Studio serves on port 1234, Ollama on 11434.
pub fn own_server(url: &str, installed: &[&'static str]) -> Option<&'static str> {
    let port = url
        .split("://")
        .last()
        .and_then(|rest| rest.split('/').next())
        .and_then(|host| host.rsplit_once(':'))
        .map(|(_, port)| port);
    match port {
        Some("1234") => Some("LM Studio"),
        Some("11434") => Some("Ollama"),
        _ => installed.first().copied(),
    }
}

/// The local servers installed in /Applications on a Mac.
pub fn installed_servers() -> Vec<&'static str> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    [
        ("/Applications/LM Studio.app", "LM Studio"),
        ("/Applications/Ollama.app", "Ollama"),
    ]
    .into_iter()
    .filter(|(path, _)| std::path::Path::new(path).exists())
    .map(|(_, name)| name)
    .collect()
}

/// Why engine `id` can't run yet and where to finish, if it can't. Codex
/// signed out has its own sign-in row.
pub fn not_ready(id: &str, settings: &AppSettings, state: &State) -> Option<&'static str> {
    match id {
        "openrouter" if state.openrouter_key.is(&false) => {
            Some("Add an OpenRouter key in Output › AI cleanup first")
        }
        "custom" if settings.dictation_processing.server.url.trim().is_empty() => {
            Some("Add your server's address in Output › AI cleanup first")
        }
        "opencode" if state.opencode_running.is(&false) => Some("Start the OpenCode app first"),
        "codex" if state.codex_signed_in.is(&false) => Some(""),
        _ => None,
    }
}

/// The built-in model `local` runs: the one chosen, else the recommended.
fn local_engine(settings: &AppSettings) -> String {
    let id = local_models::chosen(&settings.dictation_processing.server.local_model).id;
    format!("local:{id}")
}

/// What dictation's answer `id` changes, and what it keeps.
pub fn dictation(id: &str, settings: &AppSettings, state: &State) -> (Vec<Line>, Vec<String>) {
    let now = dictation_now(settings);
    if id == now {
        let kept = match id {
            "none" | "opencode" if !settings.labs.ai_cleanup_server => parts::CLEANUP,
            "none" => parts::CLEANUP,
            _ => RUNS_ON,
        };
        return (Vec::new(), vec![kept.to_owned()]);
    }
    if id == "none" {
        return (
            vec![Line::answer(
                parts::CLEANUP,
                Value::Bool(false),
                "Dictation pastes as you said it",
            )],
            Vec::new(),
        );
    }
    let why = match id {
        "local" => "Cleans up on this computer, the fastest",
        "codex" => "Cleans up dictation on your ChatGPT plan",
        "openrouter" => "Cleans up with a cloud model through OpenRouter",
        "custom" => "Cleans up on your own server",
        _ => "Cleans up through the OpenCode app",
    };
    let mut lines = Vec::new();
    let runs_on = current(settings, RUNS_ON).unwrap_or(Value::Null);
    if !(id == "opencode" && !settings.labs.ai_cleanup_server) && runs_on.as_str() != Some(id) {
        lines.push(Line::answer(RUNS_ON, Value::from(id), why));
    }
    if parts::Part::Cleanup.value(settings) != Value::Bool(true) {
        lines.push(Line::answer(
            parts::CLEANUP,
            Value::Bool(true),
            "Every dictation is cleaned up before it pastes",
        ));
    }
    let ready = not_ready(id, settings, state);
    for line in &mut lines {
        line.not_ready(ready);
    }
    (lines, Vec::new())
}

/// What the notes answer `id` changes for calls (`meetings`) and files
/// (`files`), and what it keeps.
pub fn notes(
    id: &str,
    meetings: bool,
    files: bool,
    settings: &AppSettings,
    state: &State,
) -> (Vec<Line>, Vec<String>) {
    let mut lines = Vec::new();
    let mut kept = Vec::new();
    let wanted = |now: &str| -> Value {
        match id {
            // The Codex model chosen stays.
            "codex" if now.starts_with("codex") => Value::from(now),
            "local" if now.starts_with("local:") => Value::from(now),
            "local" => Value::from(local_engine(settings)),
            id => Value::from(id),
        }
    };
    if meetings {
        if notes_engine(settings) == id {
            kept.push(if id == "apple" {
                "meetings.summaries".to_owned()
            } else {
                WRITER.to_owned()
            });
        } else if id == "apple" {
            if settings.meetings.notes.writer.is_some() {
                lines.push(Line::answer(
                    WRITER,
                    Value::from(""),
                    "Apple Intelligence writes the notes, nothing leaves this Mac",
                ));
            }
            if settings.meetings.summaries != MeetingSummaries::Mac {
                lines.push(Line::answer(
                    "meetings.summaries",
                    Value::from("mac"),
                    "Apple Intelligence writes the notes, nothing leaves this Mac",
                ));
            }
        } else {
            let now = settings.meetings.notes.writer.clone().unwrap_or_default();
            lines.push(Line::answer(WRITER, wanted(&now), notes_why(id)));
            // Every writer but OpenCode runs through the cleanup lab.
            if id != "opencode" && !settings.labs.ai_cleanup_server {
                lines.push(Line::answer(
                    SERVER_LAB,
                    Value::Bool(true),
                    "Lets meeting notes run on the engine you picked",
                ));
            }
        }
    }
    if files && id != "apple" {
        if files_engine(settings) == id {
            kept.push(FILES.to_owned());
        } else {
            let now = settings.files.cleanup_engine.clone().unwrap_or_default();
            lines.push(Line::answer(
                FILES,
                wanted(&now),
                "Cleans up transcribed files there",
            ));
        }
    }
    let ready = not_ready(id, settings, state);
    for line in &mut lines {
        line.not_ready(ready);
    }
    (lines, kept)
}

fn notes_why(id: &str) -> &'static str {
    match id {
        "codex" => "Notes on your ChatGPT plan, good with long calls",
        "local" => "Notes written on this computer, nothing leaves it",
        "openrouter" => "Notes written by a cloud model through OpenRouter",
        "custom" => "Notes written on your own server",
        _ => "Notes written through the OpenCode app",
    }
}

/// Every English reason and note this file gives, for the translation
/// tables' test.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    let mut words = vec![
        "Add an OpenRouter key in Output › AI cleanup first",
        "Add your server's address in Output › AI cleanup first",
        "Start the OpenCode app first",
        "Dictation pastes as you said it",
        "Cleans up on this computer, the fastest",
        "Cleans up dictation on your ChatGPT plan",
        "Cleans up with a cloud model through OpenRouter",
        "Cleans up on your own server",
        "Cleans up through the OpenCode app",
        "Every dictation is cleaned up before it pastes",
        "Apple Intelligence writes the notes, nothing leaves this Mac",
        "Lets meeting notes run on the engine you picked",
        "Cleans up transcribed files there",
    ];
    words.extend(["codex", "local", "openrouter", "custom", "opencode"].map(notes_why));
    words
}
