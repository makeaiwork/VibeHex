//! Meetings, through the `Library` `hex mcp` reads: the ones in the time
//! asked about, ranked by the meeting search when the question has words to
//! search for, else the newest; each read as its notes and its best
//! passages.

use std::path::Path;
use std::sync::Arc;

use super::super::intent::Intent;
use super::{Budget, Excerpt, Kind, clip, within};
use crate::meeting::query::{Library, Meeting, search_passages};

/// How much of a meeting's notes, and of one passage, a model reads.
const NOTES_CHARS: usize = 1_500;
const PASSAGE_CHARS: usize = 600;

/// How many meetings were in range, and the excerpts of those read.
pub(super) fn search(
    root: &Path,
    intent: &Intent,
    budget: Budget,
    now_ms: u64,
) -> (usize, Vec<Excerpt>) {
    let Ok(scan) = Library::new(root).scan() else {
        return (0, Vec::new());
    };
    let mut candidates: Vec<Arc<Meeting>> = scan
        .meetings
        .into_iter()
        .filter(|meeting| !meeting.in_progress)
        .filter(|meeting| overlaps(intent, meeting, now_ms))
        .collect();
    if let Some(latest) = intent.latest {
        candidates.truncate(latest);
    }
    let searched = candidates.len();
    let query = intent.terms.join(" ");
    let hits = search_passages(&candidates, &query);
    // Meetings in order of their best passage, a title naming the words
    // first; with nothing to search for, or only a few meetings asked
    // about, the newest.
    let mut picked: Vec<Arc<Meeting>> = Vec::new();
    let titled = |meeting: &Meeting| {
        let title = crate::meeting::query::fold(&meeting.title());
        intent
            .terms
            .iter()
            .any(|term| title.contains(&crate::meeting::query::fold(term)))
    };
    if query.is_empty() || intent.latest.is_some() {
        if intent.sure {
            picked.extend(candidates.iter().take(budget.meetings * 2).cloned());
        }
    } else {
        for meeting in candidates.iter().filter(|meeting| titled(meeting)) {
            picked.push(meeting.clone());
        }
        for hit in &hits {
            if !picked.iter().any(|meeting| meeting.id == hit.meeting.id) {
                picked.push(hit.meeting.clone());
            }
        }
    }
    picked.truncate(if intent.latest.is_some() || query.is_empty() {
        intent
            .latest
            .unwrap_or(budget.meetings)
            .min(budget.meetings * 2)
    } else {
        budget.meetings
    });
    let mut excerpts = Vec::new();
    for meeting in &picked {
        let title = meeting.title();
        let mut push = |offset: Option<u64>, label: &str, text: String| {
            excerpts.push(Excerpt {
                n: 0,
                kind: Kind::Meeting,
                id: meeting.id.clone(),
                title: title.clone(),
                at_ms: meeting.start_ms() + offset.unwrap_or(0),
                offset_ms: offset,
                label: label.to_owned(),
                text,
            });
        };
        let notes = meeting
            .manifest
            .notes
            .as_ref()
            .map(crate::meeting::notes_markdown)
            .filter(|notes| !notes.trim().is_empty());
        if let Some(notes) = &notes {
            push(None, "Notes", clip(notes, NOTES_CHARS));
        }
        // Its best passages, in the order they were said; a meeting
        // without notes or matches opens with its first words.
        let mut passages: Vec<_> = hits
            .iter()
            .filter(|hit| hit.meeting.id == meeting.id && !hit.notes)
            .take(budget.passages)
            .map(|hit| (hit.start_ms, hit.speaker, hit.text.clone()))
            .collect();
        if passages.is_empty() && notes.is_none() {
            passages = meeting
                .entries
                .iter()
                .take(budget.passages)
                .map(|entry| (entry.start_ms, entry.source.label(), entry.text.clone()))
                .collect();
        }
        passages.sort_by_key(|(start, _, _)| *start);
        for (start, speaker, text) in passages {
            push(Some(start), speaker, clip(&text, PASSAGE_CHARS));
        }
    }
    (searched, excerpts)
}

/// Whether the meeting was going on in the time asked about.
fn overlaps(intent: &Intent, meeting: &Meeting, now_ms: u64) -> bool {
    let start = meeting.start_ms();
    let end = start + meeting.duration_ms(now_ms);
    within(intent, start)
        || (intent.since_ms.is_none_or(|since| end >= since)
            && intent.until_ms.is_none_or(|until| start < until))
}
