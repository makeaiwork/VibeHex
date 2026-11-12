//! The transcript following the sound: the word playing lights up, a
//! line picked while paused is marked, and every line is a link that plays
//! from its start. A file's paragraph holds several segments, each its own
//! link; a meeting's turn is one; a Cleanup's sentences are links from the
//! time of their first word.

use std::ops::Range;

use rune::{El, Link, LinkPaint, Span, col, rich, row, text};

use super::super::Ui;
use super::super::assistant::jump::{self, Mark};
use super::super::controls::Paint;
use super::bar::accent_fill;
use super::timing::{self, Word};
use super::{Of, enabled, pick};
use crate::files::store::Segment;
use crate::files::text::{clock, paragraphs};
/// Where `of`'s player is, for its transcript.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in super::super) struct Heard {
    /// Where it plays or is paused; `None` before anything played or was
    /// picked.
    pub(in super::super) at_ms: Option<u64>,
    /// Paused on a picked line, which shows marked.
    pub(in super::super) marked: bool,
}

/// `of`'s transcript lines are links, and where the player is, while Labs ›
/// Listen to the recording is on.
pub(in super::super) fn heard(ui: &Ui, of: &Of) -> Option<Heard> {
    if !enabled(ui) {
        return None;
    }
    let pane = &ui.playback;
    if !pane.belongs(of) || !(pane.open || pane.marked) {
        return Some(Heard::default());
    }
    Some(Heard {
        at_ms: Some(pane.at_ms()),
        marked: pane.marked,
    })
}

/// A paragraph as shown: its start, its words, and each segment's byte
/// range in them with the segment.
type Paragraph<'a> = (u64, String, Vec<(Range<usize>, &'a Segment)>);

/// The paragraphs `segments` read as, as `files::text::paragraphs` joins
/// them.
pub(in super::super) fn file_paragraphs(segments: &[Segment]) -> Vec<Paragraph<'_>> {
    let mut spoken = segments
        .iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .peekable();
    let mut shown = Vec::new();
    for paragraph in paragraphs(segments) {
        let mut ranges = Vec::new();
        let mut offset = 0;
        while offset < paragraph.text.len() {
            let Some(segment) = spoken.next() else {
                break;
            };
            let len = segment.text.trim().len();
            ranges.push((offset..offset + len, segment));
            offset += len + 1;
        }
        shown.push((paragraph.start_ms, paragraph.text, ranges));
    }
    shown
}

/// The start of the line playing: the last one started by `at_ms`.
pub(in super::super) fn playing_line(starts: impl Iterator<Item = u64>, at_ms: u64) -> Option<u64> {
    starts.take_while(|start| *start <= at_ms).last()
}

/// The word playing or marked.
fn lit(p: Paint, word: &Word, marked: bool) -> Span {
    let span = Span::new(word.range.start, word.range.end)
        .bg(accent_fill(p, if p.k.dark { 0.28 } else { 0.2 }), 1.0)
        .color(p.k.c.text_base);
    if marked {
        span.underline(p.k.c.text_accent)
    } else {
        span
    }
}

/// `words` with `spans`, each of `links` a link that plays from its time.
fn linked(
    p: Paint,
    of: &Of,
    words: String,
    spans: Vec<Span>,
    links: impl IntoIterator<Item = (Range<usize>, u64)>,
) -> El {
    let c = &p.k.c;
    let (hover, pressed, focused) = (
        LinkPaint::new().bg(c.hover),
        LinkPaint::new().bg(c.pressed),
        LinkPaint::new().ring(c.border_focus, 1.5),
    );
    let mut line = rich(words, spans);
    for (range, start) in links {
        line = line.link(
            Link::new(range, start.to_string())
                .role("button")
                .hover(hover)
                .pressed(pressed)
                .focused(focused),
        );
    }
    let of = of.clone();
    line.on_link(move |ui: &mut Ui, id: &str, _: &mut rune::Tx| {
        if let Ok(ms) = id.parse() {
            pick(ui, &of, ms);
        }
    })
}

/// A paragraph's row: its time, in the accent while it plays, and its words.
fn paragraph_row(p: Paint, index: usize, clock_at: Option<(u64, bool)>, words: El) -> El {
    let words = words
        .key("text")
        .font(p.k.ty.ui)
        .color(p.reading())
        .grow(1.0)
        .shrink()
        .wrap();
    let mut line = row().key(format!("paragraph{index}")).gap(12.0);
    if let Some((start, here)) = clock_at {
        line = line.child(clock_el(p, start, here));
    }
    line.child(words)
}

/// A paragraph's time, in the accent while it plays.
fn clock_el(p: Paint, start: u64, here: bool) -> El {
    text(clock(start))
        .key("clock")
        .font(p.k.ty.text_xsmall_medium)
        .color(if here { p.accent() } else { p.faint() })
        .w(40.0)
        .pt(1.0)
}

/// A file's Original while it can be listened to: its paragraphs as
/// `detail` draws them, every segment a link, the word playing lit and the
/// time of its paragraph in the accent. Opened from an answer, the
/// paragraphs it cites (`marks`) are tinted with their lines lit as well.
pub(in super::super) fn file_original(
    p: Paint,
    of: &Of,
    segments: &[Segment],
    heard: Heard,
    timestamps: bool,
    marks: &[Option<Mark>],
) -> El {
    let shown = file_paragraphs(segments);
    let playing = heard.at_ms.and_then(|at| {
        playing_line(
            shown
                .iter()
                .flat_map(|(_, _, ranges)| ranges.iter().map(|(_, segment)| segment.start_ms)),
            at,
        )
    });
    let mut lines = col().key("paragraphs").gap(12.0);
    for (index, (start, words, ranges)) in shown.into_iter().enumerate() {
        let here = ranges
            .iter()
            .find(|(_, segment)| Some(segment.start_ms) == playing);
        // Over cited words, the word playing is underlined to stand out.
        let marked = heard.marked || marks.get(index).is_some_and(Option::is_some);
        let spans = here
            .zip(heard.at_ms)
            .and_then(|((range, segment), at)| {
                let line_ms = (segment.start_ms, segment.end_ms);
                let timed = timing::timed(&words, range.clone(), line_ms, &segment.words);
                timing::heard_at(&timed, at).map(|word| lit(p, word, marked))
            })
            .into_iter();
        // The word playing over the cited phrases' underlines.
        let under = marks.get(index).and_then(Option::as_ref);
        let under = under
            .map(|mark| jump::underlines(p, mark))
            .unwrap_or_default();
        let spans = under.into_iter().chain(spans).collect();
        let links: Vec<_> = ranges
            .iter()
            .map(|(range, segment)| (range.clone(), segment.start_ms))
            .collect();
        let words = linked(p, of, words, spans, links);
        if let Some(Some(mark)) = marks.get(index) {
            let lead = timestamps.then(|| clock_el(p, start, here.is_some()));
            let key = format!("paragraph{index}");
            lines = lines.child(jump::line(p, key, mark, (None, lead), words));
            continue;
        }
        let clock_at = timestamps.then_some((start, here.is_some()));
        lines = lines.child(paragraph_row(p, index, clock_at, words));
    }
    if marks.iter().any(Option::is_some) {
        lines = lines.px(jump::INSET);
    }
    lines
}

/// Whether a Cleanup of `text` can follow the sound: plain paragraphs,
/// which the Original's words can time.
pub(in super::super) fn followed(text: &str, segments: &[Segment]) -> bool {
    !segments.is_empty() && !text.trim().is_empty() && !timing::formatted(text)
}

/// A Cleanup while it can be listened to: its paragraphs, each sentence a
/// link from its first word's time, the word playing lit. Its words take
/// the times of the Original's words they match.
pub(in super::super) fn file_cleanup(
    p: Paint,
    of: &Of,
    (text, segments): (&str, &[Segment]),
    heard: Heard,
    timestamps: bool,
) -> El {
    let cleaned = timing::cleaned(text, segments);
    let now = heard.at_ms.and_then(|at| {
        let mut found = None;
        for (paragraph, (_, words)) in cleaned.paragraphs.iter().enumerate() {
            for (index, word) in words.iter().enumerate() {
                if word.at_ms > at {
                    return found.or(Some((paragraph, index)));
                }
                found = Some((paragraph, index));
            }
        }
        found
    });
    let mut lines = col().key("paragraphs").gap(12.0);
    for (index, (words, timed)) in cleaned.paragraphs.iter().enumerate() {
        let here = now.filter(|(paragraph, _)| *paragraph == index);
        let spans = here
            .and_then(|(_, word)| timed.get(word))
            .map(|word| lit(p, word, heard.marked))
            .into_iter()
            .collect();
        let line = linked(p, of, words.clone(), spans, sentences(words, timed));
        let clock_at = timestamps
            .then(|| timed.first().map(|word| (word.at_ms, here.is_some())))
            .flatten();
        lines = lines.child(paragraph_row(p, index, clock_at, line));
    }
    lines
}

/// Each sentence of `words`: its bytes and its first word's time.
fn sentences(words: &str, timed: &[Word]) -> Vec<(Range<usize>, u64)> {
    let mut shown = Vec::new();
    let mut first: Option<&Word> = None;
    for word in timed {
        let opened = *first.get_or_insert(word);
        let ends = words[word.range.clone()]
            .trim_end_matches(['"', '»', ')', '\''])
            .ends_with(['.', '?', '!', '…']);
        if ends || std::ptr::eq(word, timed.last().unwrap_or(word)) {
            shown.push((opened.range.start..word.range.end, opened.at_ms));
            first = None;
        }
    }
    shown
}

/// A meeting's turn: one link from its start, the word playing lit at
/// `starts`, its words' times when they were kept.
pub(in super::super) fn meeting_turn(
    p: Paint,
    of: &Of,
    (words, starts): (String, &[u64]),
    (start_ms, end_ms): (u64, u64),
    (lit_now, under): (Option<(u64, bool)>, Vec<Span>),
) -> El {
    let all = 0..words.len();
    let playing = lit_now.and_then(|(at, marked)| {
        let timed = timing::timed(&words, all.clone(), (start_ms, end_ms), starts);
        timing::heard_at(&timed, at).map(|word| lit(p, word, marked))
    });
    // The word playing over the cited phrases' underlines.
    let spans = under.into_iter().chain(playing).collect();
    linked(p, of, words, spans, [(all, start_ms)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start_ms: u64, text: &str) -> Segment {
        Segment {
            words: Vec::new(),
            start_ms,
            end_ms: start_ms + 1_000,
            text: text.into(),
        }
    }

    #[test]
    fn each_segment_keeps_its_range_in_its_paragraph() {
        let segments = vec![
            segment(0, " First one. "),
            segment(1_000, ""),
            segment(2_000, "Second."),
        ];
        let shown = file_paragraphs(&segments);
        assert_eq!(shown.len(), 1);
        let (start, text, ranges) = &shown[0];
        assert_eq!(*start, 0);
        assert_eq!(text, "First one. Second.");
        let kept: Vec<(Range<usize>, u64)> = ranges
            .iter()
            .map(|(range, segment)| (range.clone(), segment.start_ms))
            .collect();
        assert_eq!(kept, vec![(0..10, 0), (11..18, 2_000)]);
        assert_eq!(&text[ranges[1].0.clone()], "Second.");
    }

    #[test]
    fn the_line_playing_is_the_last_one_started() {
        let starts = [0, 4_000, 9_000];
        assert_eq!(playing_line(starts.into_iter(), 5_000), Some(4_000));
        assert_eq!(playing_line(starts.into_iter(), 9_000), Some(9_000));
        assert_eq!(playing_line([500].into_iter(), 100), None);
    }

    #[test]
    fn a_cleanups_sentences_are_links_from_their_first_word() {
        let words = "One two. Three «four»! Five";
        let timed = timing::spread(words, 0..words.len(), (0, 2_700));
        let shown = sentences(words, &timed);
        let read: Vec<&str> = shown
            .iter()
            .map(|(range, _)| &words[range.clone()])
            .collect();
        assert_eq!(read, vec!["One two.", "Three «four»!", "Five"]);
        assert_eq!(shown[1].1, timed[2].at_ms);
    }
}
