//! Meetings, through the `Library` `hex mcp` reads: the ones in the time
//! asked about, ranked by the meeting search when the question has words to
//! search for, else the newest; each read as its notes and its best
//! passages, a passage line by line as the transcript keeps them.

use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use super::super::intent::Intent;
use super::spoken::Query;
use super::{Budget, Excerpt, Kind, Line, clip, within};
use crate::meeting::TranscriptEntry;
use crate::meeting::query::{Library, Meeting, search_passages};

/// How much of a meeting's notes, and of one passage, a model reads.
const NOTES_CHARS: usize = 1_500;
const PASSAGE_CHARS: usize = 600;

/// How many meetings were in range, and the excerpts of those read.
pub(super) fn search(
    root: &Path,
    intent: &Intent,
    query: &Query,
    budget: Budget,
    now_ms: u64,
) -> (usize, Vec<Excerpt>) {
    let Ok(scan) = Library::new(root).scan() else {
        return (0, Vec::new());
    };
    // Newest first, as the library lists them. Every meeting in range is
    // searched; "the last few" are the newest of those that match.
    let candidates: Vec<Arc<Meeting>> = scan
        .meetings
        .into_iter()
        .filter(|meeting| !meeting.in_progress)
        .filter(|meeting| overlaps(intent, meeting, now_ms))
        .collect();
    let searched = candidates.len();
    let hits = search_passages(&candidates, &query.text, &query.aliases);
    // Meetings in order of their best passage, a title naming the words
    // first; with nothing to search for, the newest.
    let mut picked: Vec<Arc<Meeting>> = Vec::new();
    let titled = |meeting: &Meeting| {
        let title = crate::meeting::query::fold(&meeting.title());
        intent
            .terms
            .iter()
            .any(|term| title.contains(&crate::meeting::query::fold(term)))
    };
    let most = budget.meetings * 2;
    if query.text.is_empty() {
        if intent.sure {
            let latest = intent.latest.unwrap_or(budget.meetings).min(most);
            picked.extend(candidates.iter().take(latest).cloned());
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
        match intent.latest {
            Some(latest) => {
                picked.sort_by_key(|meeting| std::cmp::Reverse(meeting.start_ms()));
                picked.truncate(latest.min(most));
            }
            None => picked.truncate(budget.meetings),
        }
    }
    let mut excerpts = Vec::new();
    for meeting in &picked {
        let title = meeting.title();
        let mut push = |offset: Option<u64>, label: &str, text: String, lines: Vec<Line>| {
            excerpts.push(Excerpt {
                n: 0,
                kind: Kind::Meeting,
                id: meeting.id.clone(),
                title: title.clone(),
                at_ms: meeting.start_ms() + offset.unwrap_or(0),
                offset_ms: offset,
                label: label.to_owned(),
                text,
                lines,
            });
        };
        let notes = meeting
            .manifest
            .notes
            .as_ref()
            .map(crate::meeting::notes_markdown)
            .filter(|notes| !notes.trim().is_empty());
        if let Some(notes) = &notes {
            push(None, "Notes", clip(notes, NOTES_CHARS), Vec::new());
        }
        // Its best passages, in the order they were said; a meeting
        // without notes or matches opens with its first words.
        let mut passages: Vec<(u64, &str, Range<usize>)> = hits
            .iter()
            .filter(|hit| hit.meeting.id == meeting.id && !hit.notes)
            .take(budget.passages)
            .map(|hit| (hit.start_ms, hit.speaker, hit.entries.clone()))
            .collect();
        if passages.is_empty() && notes.is_none() {
            passages = meeting
                .entries
                .iter()
                .enumerate()
                .take(budget.passages)
                .map(|(index, entry)| (entry.start_ms, entry.source.label(), index..index + 1))
                .collect();
        }
        passages.sort_by_key(|(start, _, _)| *start);
        let ranges = crate::meeting::coalesced_ranges(&meeting.entries);
        for (start, speaker, entries) in passages {
            let lines = lines(&meeting.entries, &ranges, entries);
            if lines.is_empty() {
                continue;
            }
            let text = lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            push(Some(start), speaker, text, lines);
        }
    }
    (searched, excerpts)
}

/// A passage's lines, each transcript entry one, as many as a passage
/// reads (at least one).
fn lines(entries: &[TranscriptEntry], ranges: &[Range<usize>], taken: Range<usize>) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut chars = 0;
    for index in taken {
        let (Some(entry), Some(at)) = (entries.get(index), ranges.get(index)) else {
            break;
        };
        if entry.text.trim().is_empty() {
            continue;
        }
        chars += entry.text.trim().chars().count();
        if chars > PASSAGE_CHARS && !lines.is_empty() {
            break;
        }
        let times = (entry.start_ms, entry.end_ms);
        lines.push(Line::new(&entry.text, times, &entry.words, at.clone()));
    }
    lines
}

/// Whether the meeting was going on in the time asked about.
fn overlaps(intent: &Intent, meeting: &Meeting, now_ms: u64) -> bool {
    let start = meeting.start_ms();
    let end = start + meeting.duration_ms(now_ms);
    within(intent, start)
        || (intent.since_ms.is_none_or(|since| end >= since)
            && intent.until_ms.is_none_or(|until| start < until))
}
