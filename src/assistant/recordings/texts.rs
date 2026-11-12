//! Files texts and dictations, through Files' records and the History
//! snapshot, ranked by the search `hex history --search` uses.

use std::path::Path;

use super::super::intent::Intent;
use super::spoken::Query;
use super::{Budget, Excerpt, HEARD, Kind, Line, clip, newest, within};
use crate::files::store::{FileRecord, VersionKind};
use crate::meeting::query::{TextHit, search_texts};

const TEXT_CHARS: usize = 900;
/// The most of a transcript one stretch reads.
const STRETCH_CHARS: usize = 1_200;

/// How many Files records were in range, and the excerpts of those read:
/// for each, its best stretches of the transcript, segment by segment, with
/// their times. `except` is never read.
pub(super) fn files(
    dir: &Path,
    except: Option<&str>,
    intent: &Intent,
    query: &Query,
    budget: Budget,
) -> (usize, Vec<Excerpt>) {
    // Newest first, as Files lists them.
    let records: Vec<FileRecord> = crate::files::store::list_in(dir)
        .into_iter()
        .filter(|record| within(intent, record.added_ms) && Some(record.id.as_str()) != except)
        .filter(|record| !record.versions.is_empty())
        .collect();
    let searched = records.len();
    let order: Vec<(usize, f32)> = if query.text.is_empty() {
        if intent.sure {
            (0..records.len()).map(|index| (index, 0.0)).collect()
        } else {
            Vec::new()
        }
    } else {
        let documents: Vec<Vec<&str>> = records
            .iter()
            .map(|record| {
                let mut texts = vec![record.name.as_str()];
                texts.extend(record.versions.iter().map(|version| version.text.as_str()));
                texts
            })
            .collect();
        hits(search_texts(&documents, &query.text, &query.aliases))
    };
    let order = newest(order, intent, |index| records[index].added_ms);
    let mut excerpts = Vec::new();
    for record in order
        .into_iter()
        .take(budget.texts)
        .map(|index| &records[index])
    {
        let original = record
            .versions
            .iter()
            .find(|version| version.kind == VersionKind::Original);
        let segments = original
            .map(|version| version.segments.as_slice())
            .unwrap_or(&[]);
        let mut push = |offset: Option<u64>, text: String, lines: Vec<Line>| {
            excerpts.push(Excerpt {
                n: 0,
                kind: Kind::File,
                id: record.id.clone(),
                title: record.name.clone(),
                at_ms: record.added_ms,
                offset_ms: offset,
                label: String::new(),
                text,
                lines,
            });
        };
        // The stretches that match, by the same search, in order.
        let documents: Vec<Vec<&str>> = segments
            .iter()
            .map(|segment| vec![segment.text.as_str()])
            .collect();
        let mut best: Vec<usize> = if query.text.is_empty() {
            Vec::new()
        } else {
            search_texts(&documents, &query.text, &query.aliases)
                .map(|hits| hits.into_iter().map(|hit| hit.index).collect())
                .unwrap_or_default()
        };
        best.truncate(budget.passages);
        best.sort_unstable();
        if best.is_empty() {
            // Nothing timed matched: the newest version's text, from the top.
            let text = record
                .versions
                .last()
                .map(|version| version.text.as_str())
                .unwrap_or_default();
            push(None, clip(text, TEXT_CHARS), Vec::new());
        }
        // A sentence alone says little: the one before and after too, one
        // stretch where they meet.
        let ranges = crate::files::text::segment_ranges(segments);
        let mut stretches: Vec<(u64, std::ops::Range<usize>)> = Vec::new();
        for index in best {
            let around = index.saturating_sub(1)..(index + 2).min(segments.len());
            match stretches.last_mut() {
                Some((_, last)) if around.start <= last.end => last.end = around.end,
                _ => stretches.push((segments[index].start_ms, around)),
            }
        }
        for (offset, around) in stretches {
            let mut lines: Vec<Line> = Vec::new();
            let mut chars = 0;
            for index in around {
                let segment = &segments[index];
                let Some(Some(at)) = ranges.get(index) else {
                    continue;
                };
                chars += segment.text.trim().chars().count();
                if chars > STRETCH_CHARS && !lines.is_empty() {
                    break;
                }
                let times = (segment.start_ms, segment.end_ms);
                lines.push(Line::new(&segment.text, times, &segment.words, at.clone()));
            }
            let text = lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            push(Some(offset), text, lines);
        }
    }
    (searched, excerpts)
}

/// How many dictations were in range, and the ones read; `except` never.
pub(super) fn dictations(
    path: &Path,
    except: Option<&str>,
    intent: &Intent,
    query: &Query,
    budget: Budget,
) -> (usize, Vec<Excerpt>) {
    let Ok(mut entries) = crate::history::read_snapshot(path) else {
        return (0, Vec::new());
    };
    entries.retain(|entry| {
        within(intent, entry.timestamp_ms) && except != Some(entry.id.to_string().as_str())
    });
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
    // Every dictation in range is searched; "the last few" are the newest
    // of those that match, not the newest few searched.
    let searched = entries.len();
    let order: Vec<(usize, f32)> = if query.text.is_empty() {
        if intent.sure {
            (0..entries.len()).map(|index| (index, 0.0)).collect()
        } else {
            Vec::new()
        }
    } else if let Some(written) = &intent.written {
        // "Why does it write X": the dictations that wrote X.
        let documents: Vec<Vec<&str>> = entries
            .iter()
            .map(|entry| vec![entry.final_text.as_str()])
            .collect();
        let found = hits(search_texts(&documents, written, &[]));
        // Those that wrote the whole form, when any did.
        let whole: Vec<(usize, f32)> = found
            .iter()
            .copied()
            .filter(|(index, _)| entries[*index].final_text.to_lowercase().contains(written))
            .collect();
        if whole.is_empty() { found } else { whole }
    } else {
        let documents: Vec<Vec<&str>> = entries
            .iter()
            .map(|entry| vec![entry.final_text.as_str(), entry.raw_text.as_str()])
            .collect();
        hits(search_texts(&documents, &query.text, &query.aliases))
    };
    let words = matching(intent, query);
    let order = newest(order, intent, |index| entries[index].timestamp_ms);
    let excerpts = order
        .into_iter()
        .take(budget.texts * 2)
        .map(|index| {
            let entry = &entries[index];
            Excerpt {
                n: 0,
                kind: Kind::Dictation,
                id: entry.id.to_string(),
                title: entry.application.clone().unwrap_or_default(),
                at_ms: entry.timestamp_ms,
                offset_ms: None,
                label: String::new(),
                text: heard_and_wrote(&entry.raw_text, &entry.final_text, &words),
                lines: Vec::new(),
            }
        })
        .collect();
    (searched, excerpts)
}

/// The words a dictation's excerpt is cut around: the form written, the
/// terms, and their spoken forms, lowercase.
fn matching(intent: &Intent, query: &Query) -> Vec<String> {
    let mut words: Vec<String> = intent.written.iter().cloned().collect();
    words.extend(query.text.split_whitespace().map(str::to_lowercase));
    for alias in &query.aliases {
        words.push(alias.term.to_lowercase());
        words.extend(alias.sounds_like.iter().map(|form| form.to_lowercase()));
    }
    words.retain(|word| word.chars().count() >= 2);
    words
}

/// A dictation as the model reads it: what the speech model heard and what
/// was written, labelled, when cleanup changed it; else the text. Each cut
/// to its share of `TEXT_CHARS` around the first of `words` it holds, or,
/// in what was heard, where they stand in what was written.
fn heard_and_wrote(raw: &str, written: &str, words: &[String]) -> String {
    let same = |a: &str, b: &str| a.split_whitespace().eq(b.split_whitespace());
    if raw.trim().is_empty() || same(raw, written) {
        return around(written, words, TEXT_CHARS, 0.0);
    }
    let share = TEXT_CHARS / 2;
    let written = flat(written);
    let at = first(&written, words).unwrap_or(0);
    let fraction = at as f32 / written.len().max(1) as f32;
    format!(
        "{HEARD}{}\nWrote: {}",
        around(raw, words, share, fraction),
        around(&written, words, share, 0.0)
    )
}

fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where the first of `words` stands in `text`, in bytes.
fn first(text: &str, words: &[String]) -> Option<usize> {
    let lower = text.to_lowercase();
    // Lowercasing keeps byte offsets only when it keeps lengths.
    (lower.len() == text.len())
        .then(|| {
            words
                .iter()
                .filter_map(|word| lower.find(word.as_str()))
                .min()
        })
        .flatten()
}

/// `text` cut to `chars` around the first of `words` in it, with "…" where
/// it was cut; around `fraction` of the way through it when it holds none.
fn around(text: &str, words: &[String], chars: usize, fraction: f32) -> String {
    let text = flat(text);
    if text.chars().count() <= chars {
        return text;
    }
    let at = first(&text, words).unwrap_or_else(|| {
        let at = (text.len() as f32 * fraction) as usize;
        (0..=at)
            .rev()
            .find(|at| text.is_char_boundary(*at))
            .unwrap_or(0)
    });
    // A third of the room before the word.
    let before = text[..at].chars().count().saturating_sub(chars / 3);
    if before == 0 {
        return clip(&text, chars);
    }
    let start = text.char_indices().nth(before).map_or(0, |(at, _)| at);
    let start = text[start..]
        .find(' ')
        .map_or(start, |space| start + space + 1);
    format!("…{}", clip(&text[start..], chars))
}

/// The documents a search found, best first, with their scores.
fn hits(found: color_eyre::eyre::Result<Vec<TextHit>>) -> Vec<(usize, f32)> {
    found
        .map(|hits| hits.into_iter().map(|hit| (hit.index, hit.score)).collect())
        .unwrap_or_default()
}
