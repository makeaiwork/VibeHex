//! The transcript following the sound: the line playing lights up, a line
//! picked while paused is marked, and every line is a link that plays from
//! its start. A file's paragraph holds several segments, each its own link;
//! a meeting's turn is one.

use std::ops::Range;

use rune::{El, Link, LinkPaint, Span, col, rich, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::bar::accent_fill;
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
/// range in them with the segment's start.
type Paragraph = (u64, String, Vec<(Range<usize>, u64)>);

/// The paragraphs `segments` read as, as `files::text::paragraphs` joins
/// them.
pub(in super::super) fn file_paragraphs(segments: &[Segment]) -> Vec<Paragraph> {
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
            ranges.push((offset..offset + len, segment.start_ms));
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

fn link_paint(p: Paint) -> (LinkPaint, LinkPaint, LinkPaint) {
    let c = &p.k.c;
    (
        LinkPaint::new().bg(c.hover),
        LinkPaint::new().bg(c.pressed),
        LinkPaint::new().ring(c.border_focus, 1.5),
    )
}

/// The line's look while it plays or is marked.
fn lit(p: Paint, range: Range<usize>, marked: bool) -> Span {
    let span = Span::new(range.start, range.end)
        .bg(accent_fill(p, if p.k.dark { 0.24 } else { 0.18 }), 1.0)
        .color(p.k.c.text_base);
    if marked {
        span.underline(p.k.c.text_accent)
    } else {
        span
    }
}

/// A file's paragraph: each segment a link, the one playing lit.
pub(in super::super) fn file_paragraph(
    p: Paint,
    of: &Of,
    (words, ranges): (String, &[(Range<usize>, u64)]),
    playing: Option<u64>,
    marked: bool,
) -> El {
    let spans = ranges
        .iter()
        .filter(|(_, start)| Some(*start) == playing)
        .map(|(range, _)| lit(p, range.clone(), marked))
        .collect();
    let (hover, pressed, focused) = link_paint(p);
    let mut line = rich(words, spans);
    for (range, start) in ranges {
        line = line.link(
            Link::new(range.clone(), start.to_string())
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

/// A quiet tint behind the paragraph or turn that plays.
pub(in super::super) fn playing_fill(p: Paint) -> rune::Color {
    accent_fill(p, if p.k.dark { 0.08 } else { 0.06 })
}

/// A file's Original while it can be listened to: its paragraphs as
/// `detail` draws them, every segment a link, the one playing lit in a
/// tinted paragraph whose time takes the accent.
pub(in super::super) fn file_original(
    p: Paint,
    of: &Of,
    segments: &[Segment],
    heard: Heard,
    timestamps: bool,
) -> El {
    let shown = file_paragraphs(segments);
    let playing = heard.at_ms.and_then(|at| {
        playing_line(
            shown
                .iter()
                .flat_map(|(_, _, ranges)| ranges.iter().map(|(_, start)| *start)),
            at,
        )
    });
    let mut lines = col().key("paragraphs").gap(12.0);
    for (index, (start, words, ranges)) in shown.into_iter().enumerate() {
        let here = ranges.iter().any(|(_, start)| Some(*start) == playing);
        let words = file_paragraph(p, of, (words, &ranges), playing, heard.marked)
            .key("text")
            .font(p.k.ty.ui)
            .color(p.reading())
            .grow(1.0)
            .shrink()
            .wrap();
        let mut line = row().key(format!("paragraph{index}")).gap(12.0);
        if timestamps {
            line = line.child(
                text(clock(start))
                    .key("clock")
                    .font(p.k.ty.text_xsmall_medium)
                    .color(if here { p.accent() } else { p.faint() })
                    .w(40.0)
                    .pt(1.0),
            );
        }
        let line = line
            .child(words)
            .px(8.0)
            .mx(-8.0)
            .py(4.0)
            .my(-4.0)
            .radius(8.0);
        lines = lines.child(if here {
            line.fill(playing_fill(p))
        } else {
            line
        });
    }
    lines
}

/// A meeting's turn: one link from its start, lit while it plays.
pub(in super::super) fn meeting_turn(
    p: Paint,
    of: &Of,
    words: String,
    start_ms: u64,
    lit_now: Option<bool>,
) -> El {
    let range = 0..words.len();
    let spans = lit_now
        .map(|marked| vec![lit(p, range.clone(), marked)])
        .unwrap_or_default();
    let (hover, pressed, focused) = link_paint(p);
    let of = of.clone();
    rich(words, spans)
        .link(
            Link::new(range, start_ms.to_string())
                .role("button")
                .hover(hover)
                .pressed(pressed)
                .focused(focused),
        )
        .on_link(move |ui: &mut Ui, id: &str, _: &mut rune::Tx| {
            if let Ok(ms) = id.parse() {
                pick(ui, &of, ms);
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start_ms: u64, text: &str) -> Segment {
        Segment {
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
        assert_eq!(ranges, &vec![(0..10, 0), (11..18, 2_000)]);
        assert_eq!(&text[ranges[1].0.clone()], "Second.");
    }

    #[test]
    fn the_line_playing_is_the_last_one_started() {
        let starts = [0, 4_000, 9_000];
        assert_eq!(playing_line(starts.into_iter(), 5_000), Some(4_000));
        assert_eq!(playing_line(starts.into_iter(), 9_000), Some(9_000));
        assert_eq!(playing_line([500].into_iter(), 100), None);
    }
}
