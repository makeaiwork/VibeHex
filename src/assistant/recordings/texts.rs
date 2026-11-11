//! Files texts and dictations, through Files' records and the History
//! snapshot, ranked by the search `hex history --search` uses.

use std::path::Path;

use super::super::intent::Intent;
use super::spoken::Query;
use super::{Budget, Excerpt, Kind, Line, clip, newest, within};
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
    } else {
        let documents: Vec<Vec<&str>> = entries
            .iter()
            .map(|entry| vec![entry.final_text.as_str(), entry.raw_text.as_str()])
            .collect();
        hits(search_texts(&documents, &query.text, &query.aliases))
    };
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
                text: clip(&entry.final_text, TEXT_CHARS),
                lines: Vec::new(),
            }
        })
        .collect();
    (searched, excerpts)
}

/// The documents a search found, best first, with their scores.
fn hits(found: color_eyre::eyre::Result<Vec<TextHit>>) -> Vec<(usize, f32)> {
    found
        .map(|hits| hits.into_iter().map(|hit| (hit.index, hit.score)).collect())
        .unwrap_or_default()
}
