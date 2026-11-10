//! Files texts and dictations, through Files' records and the History
//! snapshot, ranked by the search `hex history --search` uses.

use std::path::Path;

use super::super::intent::Intent;
use super::{Budget, Excerpt, Kind, clip, within};
use crate::files::store::{FileRecord, VersionKind};
use crate::meeting::query::search_texts;

const TEXT_CHARS: usize = 900;
const SEGMENT_CHARS: usize = 600;

/// How many Files records were in range, and the excerpts of those read:
/// for each, its best stretches of the transcript, with their times.
pub(super) fn files(dir: &Path, intent: &Intent, budget: Budget) -> (usize, Vec<Excerpt>) {
    let mut records: Vec<FileRecord> = crate::files::store::list_in(dir)
        .into_iter()
        .filter(|record| within(intent, record.added_ms))
        .filter(|record| !record.versions.is_empty())
        .collect();
    if let Some(latest) = intent.latest {
        records.truncate(latest);
    }
    let searched = records.len();
    let query = intent.terms.join(" ");
    let order: Vec<usize> = if query.is_empty() {
        if intent.sure {
            (0..records.len()).collect()
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
        search_texts(&documents, &query, &[])
            .map(|hits| hits.into_iter().map(|hit| hit.index).collect())
            .unwrap_or_default()
    };
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
        let mut push = |offset: Option<u64>, text: String| {
            excerpts.push(Excerpt {
                n: 0,
                kind: Kind::File,
                id: record.id.clone(),
                title: record.name.clone(),
                at_ms: record.added_ms,
                offset_ms: offset,
                label: String::new(),
                text,
            });
        };
        // The stretches that match, by the same search, in order.
        let documents: Vec<Vec<&str>> = segments
            .iter()
            .map(|segment| vec![segment.text.as_str()])
            .collect();
        let mut best: Vec<usize> = if query.is_empty() {
            Vec::new()
        } else {
            search_texts(&documents, &query, &[])
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
            push(None, clip(text, TEXT_CHARS));
        }
        for index in best {
            let segment = &segments[index];
            // A sentence alone says little: the one before and after too.
            let around = segments[index.saturating_sub(1)..(index + 2).min(segments.len())]
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            push(Some(segment.start_ms), clip(&around, SEGMENT_CHARS));
        }
    }
    (searched, excerpts)
}

/// How many dictations were in range, and the ones read.
pub(super) fn dictations(path: &Path, intent: &Intent, budget: Budget) -> (usize, Vec<Excerpt>) {
    let Ok(mut entries) = crate::history::read_snapshot(path) else {
        return (0, Vec::new());
    };
    entries.retain(|entry| within(intent, entry.timestamp_ms));
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
    if let Some(latest) = intent.latest {
        entries.truncate(latest);
    }
    let searched = entries.len();
    let query = intent.terms.join(" ");
    let order: Vec<usize> = if query.is_empty() {
        if intent.sure {
            (0..entries.len()).collect()
        } else {
            Vec::new()
        }
    } else {
        let documents: Vec<Vec<&str>> = entries
            .iter()
            .map(|entry| vec![entry.final_text.as_str(), entry.raw_text.as_str()])
            .collect();
        search_texts(&documents, &query, &[])
            .map(|hits| hits.into_iter().map(|hit| hit.index).collect())
            .unwrap_or_default()
    };
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
            }
        })
        .collect();
    (searched, excerpts)
}
