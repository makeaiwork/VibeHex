//! The one place Assistant reads the person's own recordings: meetings,
//! Files texts, and dictation history. Nothing else under `assistant` or
//! `rune_ui::assistant` may (a test holds it to that).
//!
//! It is reached only through [`Allowed`], which exists only while
//! `assistant.read_recordings` is on: the person agreed once, and Assistant ›
//! Settings takes it back. It reads with the app's own readers (the
//! meetings `Library` that `hex mcp` uses, the History snapshot, Files'
//! records) and their search, on the answer's thread, never the window's;
//! what it found goes to the model as numbered excerpts the answer cites.

mod meetings;
mod texts;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::intent::Intent;
use crate::app_settings::AssistantSettings;

/// Leave to read recordings: built only from the setting being on.
#[derive(Clone, Debug)]
pub struct Allowed(());

impl Allowed {
    /// Some only when the person said yes (`assistant.read_recordings`).
    pub fn from_settings(settings: &AssistantSettings) -> Option<Self> {
        (settings.read_recordings == Some(true)).then_some(Self(()))
    }
}

/// Where the recordings are.
#[derive(Clone, Debug)]
pub struct Sources {
    meetings: PathBuf,
    history: PathBuf,
    files: PathBuf,
}

impl Sources {
    /// This account's: `<support>/meetings`, `history.json`, `files`.
    pub fn here() -> Option<Self> {
        let support = crate::app_paths::support_dir().ok()?;
        Some(Self {
            meetings: crate::meeting::root().ok()?,
            history: support.join("history.json"),
            files: support.join("files"),
        })
    }

    /// The same layout under `root`, for tests.
    #[cfg(test)]
    pub fn at(root: &std::path::Path) -> Self {
        Self {
            meetings: root.join("meetings"),
            history: root.join("history.json"),
            files: root.join("files"),
        }
    }
}

/// What a recording is.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Meeting,
    File,
    Dictation,
}

/// One numbered excerpt the model reads, `[n]` in its answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Excerpt {
    pub n: u32,
    pub kind: Kind,
    /// The meeting's folder, the Files record, or the History entry.
    pub id: String,
    pub title: String,
    /// When it was said: the passage's moment, or the recording's start.
    pub at_ms: u64,
    /// Into the meeting or file, for a passage.
    pub offset_ms: Option<u64>,
    /// `Notes`, `Me`, `Them`, or empty.
    pub label: String,
    pub text: String,
}

/// A place an answer cites, as the pane links it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Ref {
    pub n: u32,
    pub kind: Kind,
    pub id: String,
    pub title: String,
    pub at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_ms: Option<u64>,
    /// The words searched for, which the place lights when opened.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<String>,
}

/// One step of the search, as the pane lists it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    /// Meetings in the time asked about, and how many it read.
    Meetings {
        searched: usize,
        found: usize,
    },
    /// The read meetings' notes, and the passages taken from them.
    Read {
        notes: usize,
        passages: usize,
    },
    Files {
        searched: usize,
        found: usize,
    },
    Dictations {
        searched: usize,
        found: usize,
    },
}

/// How much a model may read: few for a small local one.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub meetings: usize,
    pub passages: usize,
    pub texts: usize,
    pub chars: usize,
}

impl Budget {
    pub const SMALL: Self = Self {
        meetings: 3,
        passages: 2,
        texts: 2,
        chars: 5_000,
    };
    pub const LARGE: Self = Self {
        meetings: 6,
        passages: 4,
        texts: 4,
        chars: 24_000,
    };
}

/// What the search found, in excerpt order.
#[derive(Debug, Default)]
pub struct Found {
    pub excerpts: Vec<Excerpt>,
    pub steps: Vec<Step>,
    /// The words searched for.
    pub terms: Vec<String>,
}

impl Found {
    /// The excerpts `answer` cites as `[n]`, in the order first cited.
    pub fn cited(&self, answer: &str) -> Vec<Ref> {
        let mut refs: Vec<Ref> = Vec::new();
        for n in citations(answer) {
            if refs.iter().any(|known| known.n == n) {
                continue;
            }
            if let Some(excerpt) = self.excerpts.iter().find(|excerpt| excerpt.n == n) {
                refs.push(Ref {
                    n,
                    kind: excerpt.kind,
                    id: excerpt.id.clone(),
                    title: excerpt.title.clone(),
                    at_ms: excerpt.at_ms,
                    offset_ms: excerpt.offset_ms,
                    words: self.terms.clone(),
                });
            }
        }
        refs
    }
}

/// Every `[n]` in `text`, also `[1, 3]` and `[1][3]`.
pub fn citations(text: &str) -> Vec<u32> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find(']') else { break };
        let inside = &rest[..close];
        if !inside.is_empty()
            && inside
                .chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == ' ')
        {
            found.extend(
                inside
                    .split(',')
                    .filter_map(|n| n.trim().parse::<u32>().ok()),
            );
        }
        rest = &rest[close + 1..];
    }
    found
}

/// Searches the recordings `intent` asks about, reporting each step as it
/// lands. A question that only may be about recordings finds nothing when
/// nothing matches its words.
pub fn search(
    _allowed: &Allowed,
    sources: &Sources,
    intent: &Intent,
    budget: Budget,
    now_ms: u64,
    step: &mut dyn FnMut(&Step),
) -> Found {
    let mut found = Found {
        terms: intent.terms.clone(),
        ..Found::default()
    };
    let mut record = |found: &mut Found, done: Step| {
        step(&done);
        found.steps.push(done);
    };
    let mut excerpts = Vec::new();
    if intent.kinds.meetings {
        let (searched, picked) = meetings::search(&sources.meetings, intent, budget, now_ms);
        let read = picked
            .iter()
            .map(|excerpt| &excerpt.id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        record(
            &mut found,
            Step::Meetings {
                searched,
                found: read,
            },
        );
        if read > 0 {
            let notes = picked
                .iter()
                .filter(|excerpt| excerpt.label == "Notes")
                .count();
            record(
                &mut found,
                Step::Read {
                    notes,
                    passages: picked.len() - notes,
                },
            );
        }
        excerpts.extend(picked);
    }
    if intent.kinds.files {
        let (searched, picked) = texts::files(&sources.files, intent, budget);
        let read = distinct(&picked);
        record(
            &mut found,
            Step::Files {
                searched,
                found: read,
            },
        );
        excerpts.extend(picked);
    }
    if intent.kinds.dictations {
        let (searched, picked) = texts::dictations(&sources.history, intent, budget);
        let read = picked.len();
        record(
            &mut found,
            Step::Dictations {
                searched,
                found: read,
            },
        );
        excerpts.extend(picked);
    }
    // Number in order and keep to the budget: a meeting's notes and its
    // best passages first, as they came.
    let mut chars = 0;
    for mut excerpt in excerpts {
        chars += excerpt.text.len();
        if chars > budget.chars && !found.excerpts.is_empty() {
            break;
        }
        excerpt.n = found.excerpts.len() as u32 + 1;
        found.excerpts.push(excerpt);
    }
    found
}

fn distinct(excerpts: &[Excerpt]) -> usize {
    excerpts
        .iter()
        .map(|excerpt| &excerpt.id)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// `text` cut to about `chars`, at a word.
fn clip(text: &str, chars: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match text.char_indices().nth(chars) {
        Some((cut, _)) => {
            let cut = text[..cut].rfind(' ').unwrap_or(cut);
            format!("{}…", &text[..cut])
        }
        None => text,
    }
}

/// Whether `range` holds `at_ms`.
fn within(intent: &Intent, at_ms: u64) -> bool {
    intent.since_ms.is_none_or(|since| at_ms >= since)
        && intent.until_ms.is_none_or(|until| at_ms < until)
}

#[cfg(test)]
mod tests;
