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
//! A transcript's excerpt numbers its lines too (`[2.3]`), so an answer
//! cites the very lines it rests on and the place opens with them lit.

pub mod cite;
mod meetings;
mod texts;

use std::ops::Range;
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
    /// What the model reads: its lines joined, when it has them.
    pub text: String,
    /// A transcript's lines, `[n.1]` on: none for notes, a dictation, or
    /// a text read whole, which are cited whole.
    pub lines: Vec<Line>,
}

impl Excerpt {
    /// A meeting's notes, which the answer cites whole.
    pub fn is_notes(&self) -> bool {
        self.kind == Kind::Meeting && self.label == NOTES
    }
}

/// The label of a meeting's notes as an excerpt.
pub(crate) const NOTES: &str = "Notes";

/// One line of a transcript excerpt: a sentence or a segment, as the
/// transcription model gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    /// From its first word to its last, into the recording: the word's
    /// own times where the model kept them, else the segment's.
    pub start_ms: u64,
    pub end_ms: u64,
    /// Its bytes in the item's text as its pane shows it: a meeting's
    /// turns one per line, a Files Original's paragraphs.
    pub at: Range<usize>,
}

impl Line {
    /// `text` from `start_ms` to `end_ms`, at `at`, starting at its first
    /// timed word when `words` time it.
    fn new(text: &str, (start_ms, end_ms): (u64, u64), words: &[u64], at: Range<usize>) -> Self {
        let first = crate::meeting::word_starts(text, words).and_then(|words| words.first());
        Self {
            text: text.trim().to_owned(),
            start_ms: first.copied().unwrap_or(start_ms).max(start_ms),
            end_ms: end_ms.max(start_ms),
            at,
        }
    }
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
    /// The lines its marks cite, which the place lights when opened; none
    /// when they named no line it has, and then only the passage shows.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Lines>,
    /// It is a meeting's notes, not its transcript: it opens on them.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub notes: bool,
}

/// Lines `[n.first-n.last]` one of the answer's marks cites, mapped to
/// where they are.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Lines {
    /// Which of the answer's `[n]` for this excerpt cites them, from 0.
    pub mark: u32,
    pub first: u32,
    pub last: u32,
    /// From the first line's first word to the last line's last, into the
    /// recording.
    pub start_ms: u64,
    pub end_ms: u64,
    /// Their bytes in the item's text (see [`Line::at`]).
    pub at: Range<usize>,
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
}

impl Found {
    /// The excerpts `answer` cites, in the order first cited, each with
    /// the lines its marks name. Lines an excerpt doesn't have are left
    /// out, and so are excerpts the search didn't find.
    pub fn cited(&self, answer: &str) -> Vec<Ref> {
        let mut refs: Vec<Ref> = Vec::new();
        // How many marks each excerpt has had so far.
        let mut marks: Vec<(u32, u32)> = Vec::new();
        for run in cite::runs(answer) {
            for n in run.numbers() {
                let mark = match marks.iter_mut().find(|(known, _)| *known == n) {
                    Some((_, count)) => {
                        *count += 1;
                        *count - 1
                    }
                    None => {
                        marks.push((n, 1));
                        0
                    }
                };
                let Some(excerpt) = self.excerpts.iter().find(|excerpt| excerpt.n == n) else {
                    continue;
                };
                let at = match refs.iter().position(|known| known.n == n) {
                    Some(at) => at,
                    None => {
                        refs.push(Ref {
                            n,
                            kind: excerpt.kind,
                            id: excerpt.id.clone(),
                            title: excerpt.title.clone(),
                            at_ms: excerpt.at_ms,
                            offset_ms: excerpt.offset_ms,
                            lines: Vec::new(),
                            notes: excerpt.is_notes(),
                        });
                        refs.len() - 1
                    }
                };
                for (first, last) in run
                    .cites
                    .iter()
                    .filter(|c| c.n == n)
                    .filter_map(|c| c.lines)
                {
                    if let Some(lines) = excerpt.lines_of(mark, first, last)
                        && !refs[at].lines.contains(&lines)
                    {
                        refs[at].lines.push(lines);
                    }
                }
            }
        }
        refs
    }
}

impl Excerpt {
    /// Its lines `first..=last` as one of the answer's marks cites them;
    /// none when it hasn't them all.
    fn lines_of(&self, mark: u32, first: u32, last: u32) -> Option<Lines> {
        let index = |id: u32| id.checked_sub(1).map(|index| index as usize);
        let from = self.lines.get(index(first)?)?;
        let to = self.lines.get(index(last)?)?;
        Some(Lines {
            mark,
            first,
            last,
            start_ms: from.start_ms,
            end_ms: to.end_ms.max(from.start_ms),
            at: from.at.start..to.at.end.max(from.at.start),
        })
    }
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
    let mut found = Found::default();
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
