//! What a model reads of the recordings when it asks for it itself
//! (`assistant::agent`): a search it words, the list of meetings or Files
//! texts, one meeting's notes, a stretch of a transcript, one dictation,
//! and what the use shows. Every function here takes [`Allowed`], as the
//! search does, and only reads: the meetings `Library`, Files' records,
//! and the History snapshot, none of which write.
//!
//! What comes back is excerpts for the turn's [`super::Shelf`], numbered
//! there, or a list of ids and titles. A transcript's excerpt keeps its
//! lines with their times and places, so an answer cites `[n.m]` and the
//! place opens as it does for a search's passage.

use std::ops::Range;
use std::sync::Arc;

use super::super::intent::{Intent, Kinds};
use super::{Allowed, Budget, Excerpt, Found, HEARD, Kind, Line, NOTES, Sources, Step, usage};
use crate::dictation::vocabulary::VocabularyEntry;
use crate::meeting::query::{Library, ListQuery, Meeting, OffsetRange, iso_local, parse_time};

/// The most of a transcript one read returns; the rest is asked for from
/// where it stopped.
pub const TRANSCRIPT_CHARS: usize = 4_000;
/// The most of a meeting's notes, or of a text without times, one read
/// returns.
const WHOLE_CHARS: usize = 6_000;
/// The most meetings or files a list names.
pub const MOST_LISTED: usize = 20;

/// What a model's search names: its words, the kinds, a time as
/// `meeting::query::parse_time` reads it (`2026-11-04`, `yesterday`, `7d`),
/// and the newest few.
#[derive(Clone, Debug, Default)]
pub struct Asked {
    pub query: String,
    pub kinds: Kinds,
    pub since: Option<String>,
    pub until: Option<String>,
    pub latest: Option<usize>,
}

/// A time bound as milliseconds; an upper one covers its day.
fn bound(text: Option<&str>, now_ms: u64, upper: bool) -> Result<Option<u64>, String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| parse_time(text, now_ms, upper).map_err(|error| format!("{error:#}")))
        .transpose()
}

/// Searches the recordings for what `asked` names, as a typed question's
/// search does (`super::search`), each step reported as it lands.
pub fn search(
    allowed: &Allowed,
    sources: &Sources,
    asked: &Asked,
    vocabulary: &[VocabularyEntry],
    (budget, now_ms): (Budget, u64),
    step: &mut dyn FnMut(&Step),
) -> Result<Found, String> {
    let since = bound(asked.since.as_deref(), now_ms, false)?;
    let until = bound(asked.until.as_deref(), now_ms, true)?;
    if since
        .zip(until)
        .is_some_and(|(since, until)| since >= until)
    {
        return Err("since must be earlier than until".into());
    }
    let intent = Intent::asked(asked.kinds, &asked.query, (since, until), asked.latest);
    Ok(super::search(
        allowed, sources, &intent, vocabulary, budget, now_ms, step,
    ))
}

/// A list a model reads to choose from, and how much was looked through.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listed {
    pub text: String,
    pub searched: usize,
    pub found: usize,
}

/// `2026-11-04 10:12`, local.
fn stamp(ms: u64) -> String {
    let iso = iso_local(ms);
    iso.get(..16).unwrap_or(&iso).replace('T', " ")
}

/// `12:30`, `1:02:03`.
pub fn clock(ms: u64) -> String {
    let seconds = ms / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match hours {
        0 => format!("{minutes:02}:{seconds:02}"),
        hours => format!("{hours}:{minutes:02}:{seconds:02}"),
    }
}

/// The same to the millisecond, where it isn't a whole second: where a
/// read stopped, so the next starts on the very entry left out.
fn exact(ms: u64) -> String {
    match ms % 1000 {
        0 => clock(ms),
        part => format!("{}.{part:03}", clock(ms)),
    }
}

/// The meetings in a time, newest first, at most `limit`: `id | title |
/// start | length | notes`. `query` narrows by words of the title.
pub fn meetings_list(
    _allowed: &Allowed,
    sources: &Sources,
    (since, until, query): (Option<&str>, Option<&str>, Option<&str>),
    limit: usize,
    now_ms: u64,
) -> Result<Listed, String> {
    let library = Library::new(&sources.meetings);
    let all = library
        .scan()
        .map_err(|error| format!("{error:#}"))?
        .meetings
        .iter()
        .filter(|meeting| !meeting.in_progress)
        .count();
    let listed = library
        .list_matches(
            &ListQuery {
                since: since.map(str::to_owned),
                until: until.map(str::to_owned),
                query: query.map(str::to_owned),
                limit,
                include_in_progress: false,
            },
            now_ms,
        )
        .map_err(|error| format!("{error:#}"))?;
    let except = sources.except(Kind::Meeting);
    let matching: Vec<&Arc<Meeting>> = listed
        .meetings
        .iter()
        .filter(|meeting| Some(meeting.id.as_str()) != except)
        .collect();
    let limit = limit.clamp(1, MOST_LISTED);
    let mut text = match matching.len() {
        0 => "No meetings match.\n".to_owned(),
        found => format!(
            "{} of {found} meetings, newest first:\nid | title | start | length | notes\n",
            found.min(limit)
        ),
    };
    for meeting in matching.iter().take(limit) {
        text.push_str(&format!(
            "{} | {} | {} | {} | {}\n",
            meeting.id,
            meeting.title(),
            stamp(meeting.start_ms()),
            clock(meeting.duration_ms(now_ms)),
            match meeting.manifest.notes {
                Some(_) => "notes",
                None => "no notes",
            }
        ));
    }
    Ok(Listed {
        text,
        searched: all,
        found: matching.len().min(limit),
    })
}

/// The meeting `id` names: its id, or the start of it when only one
/// meeting starts so. Never one still being recorded, nor the one a
/// question was asked about.
fn meeting(sources: &Sources, id: &str) -> Result<Arc<Meeting>, String> {
    let no = || format!("No meeting has the id {id:?}; meetings_list gives the ids.");
    let meeting = Library::new(&sources.meetings)
        .meeting(id)
        .map_err(|_| no())?;
    if meeting.in_progress || sources.except(Kind::Meeting) == Some(meeting.id.as_str()) {
        return Err(no());
    }
    Ok(meeting)
}

/// `text` whole, or cut at `chars` on a line's end, saying it was cut.
fn capped(text: &str, chars: usize) -> String {
    let text = text.trim();
    match text.char_indices().nth(chars) {
        Some((cut, _)) => {
            let cut = text[..cut].rfind('\n').unwrap_or(cut);
            format!("{}\n… (cut)", text[..cut].trim_end())
        }
        None => text.to_owned(),
    }
}

/// Meeting `id`'s notes, as one excerpt the answer cites whole.
pub fn meeting_notes(_allowed: &Allowed, sources: &Sources, id: &str) -> Result<Excerpt, String> {
    let meeting = meeting(sources, id)?;
    let notes = meeting
        .manifest
        .notes
        .as_ref()
        .map(crate::meeting::notes_markdown)
        .filter(|notes| !notes.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "The meeting \"{}\" has no notes; meeting_transcript reads what was said.",
                meeting.title()
            )
        })?;
    Ok(Excerpt {
        n: 0,
        kind: Kind::Meeting,
        id: meeting.id.clone(),
        title: meeting.title(),
        at_ms: meeting.start_ms(),
        offset_ms: None,
        label: NOTES.to_owned(),
        text: capped(&notes, WHOLE_CHARS),
        lines: Vec::new(),
    })
}

/// A stretch of a recording read in order, and where the next one starts
/// when there is more.
#[derive(Clone, Debug, PartialEq)]
pub struct Stretch {
    pub excerpt: Excerpt,
    /// The offset to read on from, as `from` takes it (`12:30`).
    pub more: Option<String>,
}

/// One timed piece of a transcript: who said it (empty when it isn't
/// known), its words, its times, its words' times, and where it stands in
/// the text its pane shows.
struct Piece<'a> {
    speaker: &'a str,
    text: &'a str,
    times: (u64, u64),
    words: &'a [u64],
    at: Range<usize>,
}

/// The lines of `pieces`, in order, to about `TRANSCRIPT_CHARS`, each
/// speaker named where they start to speak; and the start of the first
/// piece left out.
fn lines<'a>(pieces: impl Iterator<Item = Piece<'a>>) -> (Vec<Line>, Option<u64>) {
    let mut lines: Vec<Line> = Vec::new();
    let mut chars = 0;
    let mut speaker = "";
    for piece in pieces {
        if piece.text.trim().is_empty() {
            continue;
        }
        chars += piece.text.trim().chars().count();
        if chars > TRANSCRIPT_CHARS && !lines.is_empty() {
            return (lines, Some(piece.times.0));
        }
        let mut line = Line::new(piece.text, piece.times, piece.words, piece.at);
        if piece.speaker != speaker {
            speaker = piece.speaker;
            line.text = format!("{speaker}: {}", line.text);
        }
        lines.push(line);
    }
    (lines, None)
}

fn stretch(
    (kind, id, title): (Kind, &str, String),
    start_ms: u64,
    (lines, more): (Vec<Line>, Option<u64>),
) -> Stretch {
    let from = lines.first().map_or(0, |line| line.start_ms);
    let to = lines.last().map_or(from, |line| line.end_ms);
    let text = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Stretch {
        excerpt: Excerpt {
            n: 0,
            kind,
            id: id.to_owned(),
            title,
            at_ms: start_ms + from,
            offset_ms: Some(from),
            label: format!("transcript {} to {}", clock(from), clock(to)),
            text,
            lines,
        },
        more: more.map(exact),
    }
}

/// Meeting `id`'s transcript from `from` to `to` (offsets into it, as
/// `meeting::query::parse_offset` reads them; open when none), a line an
/// entry, to about [`TRANSCRIPT_CHARS`].
pub fn meeting_transcript(
    _allowed: &Allowed,
    sources: &Sources,
    id: &str,
    (from, to): (Option<&str>, Option<&str>),
    now_ms: u64,
) -> Result<Stretch, String> {
    let meeting = meeting(sources, id)?;
    let range = OffsetRange::parse(from, to).map_err(|error| format!("{error:#}"))?;
    // Says why when the range starts past the meeting's end.
    meeting
        .entries_in(range, now_ms)
        .map_err(|error| format!("{error:#}"))?;
    let ranges = crate::meeting::coalesced_ranges(&meeting.entries);
    let pieces = meeting
        .entries
        .iter()
        .zip(&ranges)
        .filter(|(entry, _)| {
            range.from_ms.is_none_or(|from| entry.end_ms > from)
                && range.to_ms.is_none_or(|to| entry.start_ms < to)
        })
        .map(|(entry, at)| Piece {
            speaker: entry.source.label(),
            text: &entry.text,
            times: (entry.start_ms, entry.end_ms),
            words: &entry.words,
            at: at.clone(),
        });
    let read = lines(pieces);
    if read.0.is_empty() {
        return Err(format!(
            "Nothing was said in that part of \"{}\".",
            meeting.title()
        ));
    }
    Ok(stretch(
        (Kind::Meeting, &meeting.id, meeting.title()),
        meeting.start_ms(),
        read,
    ))
}

/// Files' records, newest first, at most [`MOST_LISTED`]: `id | name |
/// added | length`. `query` narrows by words of the name.
pub fn files_list(_allowed: &Allowed, sources: &Sources, query: Option<&str>) -> Listed {
    let except = sources.except(Kind::File);
    let records: Vec<_> = crate::files::store::list_in(&sources.files)
        .into_iter()
        .filter(|record| !record.versions.is_empty() && Some(record.id.as_str()) != except)
        .collect();
    let words: Vec<String> = query
        .unwrap_or_default()
        .split_whitespace()
        .map(crate::meeting::query::fold)
        .collect();
    let matching: Vec<_> = records
        .iter()
        .filter(|record| {
            let name = crate::meeting::query::fold(&record.name);
            words.iter().all(|word| name.contains(word.as_str()))
        })
        .collect();
    let mut text = match matching.len() {
        0 => "No files match.\n".to_owned(),
        found => format!(
            "{} of {found} files, newest first:\nid | name | added | length\n",
            found.min(MOST_LISTED)
        ),
    };
    for record in matching.iter().take(MOST_LISTED) {
        text.push_str(&format!(
            "{} | {} | {} | {}\n",
            record.id,
            record.name,
            stamp(record.added_ms),
            clock(record.duration_ms)
        ));
    }
    Listed {
        text,
        searched: records.len(),
        found: matching.len().min(MOST_LISTED),
    }
}

/// Files text `id` from `from` to `to`: its Original's timed transcript,
/// a line a segment, or, for a text without times, the newest version
/// from the top.
pub fn file_read(
    _allowed: &Allowed,
    sources: &Sources,
    id: &str,
    (from, to): (Option<&str>, Option<&str>),
) -> Result<Stretch, String> {
    let no = || format!("No file has the id {id:?}; files_list gives the ids.");
    let record = crate::files::store::list_in(&sources.files)
        .into_iter()
        .find(|record| record.id == id.trim())
        .filter(|record| sources.except(Kind::File) != Some(record.id.as_str()))
        .ok_or_else(no)?;
    let range = OffsetRange::parse(from, to).map_err(|error| format!("{error:#}"))?;
    let segments = record
        .versions
        .iter()
        .find(|version| version.kind == crate::files::store::VersionKind::Original)
        .map(|version| version.segments.as_slice())
        .unwrap_or(&[]);
    let ranges = crate::files::text::segment_ranges(segments);
    let pieces = segments
        .iter()
        .zip(&ranges)
        .filter(|(segment, _)| {
            range.from_ms.is_none_or(|from| segment.end_ms > from)
                && range.to_ms.is_none_or(|to| segment.start_ms < to)
        })
        .filter_map(|(segment, at)| {
            Some(Piece {
                speaker: "",
                text: &segment.text,
                times: (segment.start_ms, segment.end_ms),
                words: &segment.words,
                at: at.clone()?,
            })
        });
    let read = lines(pieces);
    if !read.0.is_empty() {
        return Ok(stretch(
            (Kind::File, &record.id, record.name.clone()),
            record.added_ms,
            read,
        ));
    }
    if !segments.is_empty() {
        return Err(format!("Nothing in that part of \"{}\".", record.name));
    }
    // No times: the newest version's text, which is cited whole.
    let text = record
        .versions
        .last()
        .map(|version| version.text.as_str())
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err(format!("\"{}\" has no text yet.", record.name));
    }
    Ok(Stretch {
        excerpt: Excerpt {
            n: 0,
            kind: Kind::File,
            id: record.id.clone(),
            title: record.name.clone(),
            at_ms: record.added_ms,
            offset_ms: None,
            label: String::new(),
            text: capped(text, WHOLE_CHARS),
            lines: Vec::new(),
        },
        more: None,
    })
}

/// Dictation `id` whole: what was heard and what was written when cleanup
/// changed it, else its text.
pub fn dictation_read(_allowed: &Allowed, sources: &Sources, id: &str) -> Result<Excerpt, String> {
    let no = || format!("No dictation has the id {id:?}; recordings_search finds dictations.");
    let entries = crate::history::read_snapshot(&sources.history).map_err(|_| no())?;
    let entry = entries
        .iter()
        .find(|entry| entry.id.to_string() == id.trim())
        .filter(|entry| sources.except(Kind::Dictation) != Some(entry.id.to_string().as_str()))
        .ok_or_else(no)?;
    let same = entry
        .raw_text
        .split_whitespace()
        .eq(entry.final_text.split_whitespace());
    let text = if entry.raw_text.trim().is_empty() || same {
        capped(&entry.final_text, TRANSCRIPT_CHARS)
    } else {
        format!(
            "{HEARD}{}\nWrote: {}",
            capped(&entry.raw_text, TRANSCRIPT_CHARS / 2),
            capped(&entry.final_text, TRANSCRIPT_CHARS / 2)
        )
    };
    Ok(Excerpt {
        n: 0,
        kind: Kind::Dictation,
        id: entry.id.to_string(),
        title: entry.application.clone().unwrap_or_default(),
        at_ms: entry.timestamp_ms,
        offset_ms: None,
        label: String::new(),
        text,
        lines: Vec::new(),
    })
}

/// What the person's use shows, as the lines a returning setup reads
/// (`usage::lines`): counts, shares, apps, and the single words cleanup
/// keeps fixing; never a dictation's or a meeting's text. `known` is what
/// Vocabulary has.
pub fn usage_lines(
    allowed: &Allowed,
    sources: &Sources,
    known: &[String],
    now_ms: u64,
) -> Vec<String> {
    usage::lines(&usage::read(allowed, sources, known, now_ms))
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
