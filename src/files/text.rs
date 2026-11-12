//! A file's text as people read and copy it: the timed transcript in
//! paragraphs (with or without their times), and Markdown as plain text.

use super::store::Segment;

/// A paragraph ends at a pause this long once it has run this long…
const PAUSE_MS: u64 = 1_500;
const SETTLED_MS: u64 = 15_000;
/// …and anyway at this length.
const LONGEST_MS: u64 = 45_000;

/// One paragraph of the transcript and when it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paragraph {
    pub start_ms: u64,
    pub text: String,
}

/// The timed transcript in paragraphs: a new one after a pause once the
/// last has run a while, or once it has run long.
pub fn paragraphs(segments: &[Segment]) -> Vec<Paragraph> {
    laid_out(segments).0
}

/// Where each segment's words are in [`plain`]: none for a silent one.
pub fn segment_ranges(segments: &[Segment]) -> Vec<Option<std::ops::Range<usize>>> {
    laid_out(segments).1
}

fn laid_out(segments: &[Segment]) -> (Vec<Paragraph>, Vec<Option<std::ops::Range<usize>>>) {
    let mut paragraphs: Vec<Paragraph> = Vec::new();
    let mut ranges = Vec::new();
    // Where the last paragraph starts in the plain text.
    let mut base = 0;
    let mut last_end = 0;
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            ranges.push(None);
            continue;
        }
        let new = match paragraphs.last() {
            None => true,
            Some(paragraph) => {
                let length = segment.start_ms.saturating_sub(paragraph.start_ms);
                let pause = segment.start_ms.saturating_sub(last_end);
                length >= LONGEST_MS || (length >= SETTLED_MS && pause >= PAUSE_MS)
            }
        };
        if new {
            if let Some(last) = paragraphs.last() {
                base += last.text.len() + PARAGRAPH_BREAK.len();
            }
            ranges.push(Some(base..base + text.len()));
            paragraphs.push(Paragraph {
                start_ms: segment.start_ms,
                text: text.into(),
            });
        } else if let Some(paragraph) = paragraphs.last_mut() {
            paragraph.text.push(' ');
            let at = base + paragraph.text.len();
            ranges.push(Some(at..at + text.len()));
            paragraph.text.push_str(text);
        }
        last_end = segment.end_ms;
    }
    (paragraphs, ranges)
}

/// What [`plain`] puts between paragraphs.
pub const PARAGRAPH_BREAK: &str = "\n\n";

/// The paragraphs as plain text.
pub fn plain(segments: &[Segment]) -> String {
    paragraphs(segments)
        .into_iter()
        .map(|paragraph| paragraph.text)
        .collect::<Vec<_>>()
        .join(PARAGRAPH_BREAK)
}

/// The paragraphs, each after its time: `00:21 text`.
pub fn timestamped(segments: &[Segment]) -> String {
    paragraphs(segments)
        .into_iter()
        .map(|paragraph| format!("{} {}", clock(paragraph.start_ms), paragraph.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `mm:ss`, or `h:mm:ss` from an hour.
pub fn clock(ms: u64) -> String {
    let seconds = ms / 1_000;
    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// A length for the list: `42 min`, `1 h 12 min`, `40 s`.
pub fn length(ms: u64) -> String {
    use crate::desktop::i18n::{tr_fill, tr_named};
    let minutes = (ms + 30_000) / 60_000;
    match minutes {
        0 => tr_fill("{} s", &(ms / 1_000).to_string()),
        1..=59 => tr_fill("{} min", &minutes.to_string()),
        _ if minutes.is_multiple_of(60) => tr_fill("{} h", &(minutes / 60).to_string()),
        _ => tr_named(
            "{hours} h {minutes} min",
            &[
                ("hours", &(minutes / 60).to_string()),
                ("minutes", &(minutes % 60).to_string()),
            ],
        ),
    }
}

/// Markdown as plain text: no heading marks, emphasis, code fences, link
/// targets, or quote marks; list items keep a dash or their number.
pub fn strip_markdown(markdown: &str) -> String {
    let mut lines = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || is_rule(trimmed) {
            continue;
        }
        let indent = &line[..line.len() - trimmed.len()];
        let mut rest = trimmed.trim_start_matches('#');
        if rest.len() != trimmed.len() {
            rest = rest.trim_start();
        }
        rest = rest.strip_prefix("> ").unwrap_or(rest);
        let (bullet, body) = if let Some(item) = rest
            .strip_prefix("- [ ] ")
            .or_else(|| rest.strip_prefix("- [x] "))
            .or_else(|| rest.strip_prefix("* "))
            .or_else(|| rest.strip_prefix("+ "))
            .or_else(|| rest.strip_prefix("- "))
        {
            ("- ", item)
        } else {
            ("", rest)
        };
        let body = if body.starts_with('|') {
            table_row(body)
        } else {
            inline(body)
        };
        if body.is_empty() && bullet.is_empty() && !rest.is_empty() {
            continue;
        }
        lines.push(format!("{indent}{bullet}{body}"));
    }
    lines.join("\n").trim().to_string()
}

/// A `---`, `***`, or `___` line.
fn is_rule(line: &str) -> bool {
    let line = line.trim();
    line.len() >= 3
        && (line.chars().all(|c| c == '-' || c == ' ')
            || line.chars().all(|c| c == '*' || c == ' ')
            || line.chars().all(|c| c == '_' || c == ' '))
}

/// `| a | b |` → `a, b`; a `|---|` separator row is dropped.
fn table_row(row: &str) -> String {
    let cells: Vec<String> = row
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| inline(cell.trim()))
        .collect();
    if cells
        .iter()
        .all(|cell| cell.chars().all(|c| c == '-' || c == ':'))
    {
        return String::new();
    }
    cells.join(", ")
}

/// Emphasis and code marks dropped, `[text](url)` as its text.
fn inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        if c == '['
            && let Some(close) = chars[index..].iter().position(|&c| c == ']')
            && chars.get(index + close + 1) == Some(&'(')
            && let Some(end) = chars[index + close..].iter().position(|&c| c == ')')
        {
            out.extend(&chars[index + 1..index + close]);
            index += close + end + 1;
            continue;
        }
        let emphasis = (c == '*' || c == '`' || c == '~')
            || (c == '_'
                && !chars
                    .get(index.wrapping_sub(1))
                    .is_some_and(|c| c.is_alphanumeric())
                || c == '_' && !chars.get(index + 1).is_some_and(|c| c.is_alphanumeric()));
        if !emphasis {
            out.push(c);
        }
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start_ms: u64, end_ms: u64, text: &str) -> Segment {
        Segment {
            words: Vec::new(),
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    #[test]
    fn paragraphs_break_at_a_pause_once_settled_and_at_length() {
        let segments = [
            segment(0, 5_000, "One."),
            segment(5_200, 10_000, "Two."),
            // A pause, but the paragraph is only 10 s long.
            segment(12_000, 16_000, "Three."),
            // A pause after 16 s: a new paragraph.
            segment(18_000, 30_000, "Four."),
            segment(30_100, 62_000, "Five."),
            // 45 s from Four with no pause: still a new one.
            segment(63_100, 64_000, "Six."),
        ];
        let paragraphs = paragraphs(&segments);
        let texts: Vec<&str> = paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["One. Two. Three.", "Four. Five.", "Six."]);
        assert_eq!(paragraphs[1].start_ms, 18_000);
        assert_eq!(
            timestamped(&segments[..1]),
            "00:00 One.",
            "times lead each paragraph"
        );
    }

    /// Assistant lights a cited segment by where its words are in the text.
    #[test]
    fn each_segment_knows_where_its_words_are_in_the_plain_text() {
        let segments = vec![
            segment(0, 5_000, " Один. "),
            segment(5_200, 10_000, ""),
            segment(10_100, 16_000, "Two."),
            segment(20_000, 30_000, "Three."),
        ];
        let text = plain(&segments);
        let read: Vec<Option<&str>> = segment_ranges(&segments)
            .into_iter()
            .map(|range| range.map(|range| &text[range]))
            .collect();
        assert_eq!(read, [Some("Один."), None, Some("Two."), Some("Three.")]);
        assert_eq!(text, "Один. Two.\n\nThree.");
    }

    #[test]
    fn clocks_and_lengths_read_like_the_mockups() {
        assert_eq!(clock(21_000), "00:21");
        assert_eq!(clock(3_725_000), "1:02:05");
        assert_eq!(length(42 * 60_000 + 10_000), "42 min");
        assert_eq!(length(72 * 60_000), "1 h 12 min");
        assert_eq!(length(40_000), "1 min");
        assert_eq!(length(20_000), "20 s");
    }

    #[test]
    fn markdown_reads_as_plain_text() {
        let markdown = "## Конспект\n\n### Как сейчас\n- Отчёты **вручную**: CRM → Excel\n\
                        1. Пилот *до 28 ноября*\n- [ ] Позвонить\n> цитата\n\n---\n\
                        См. [документ](https://example.com) и `код`.\n\
                        | Проблема | Срок |\n|---|---|\n| Отчёты | пятница |\n\
                        snake_case stays";
        assert_eq!(
            strip_markdown(markdown),
            "Конспект\n\nКак сейчас\n- Отчёты вручную: CRM → Excel\n1. Пилот до 28 ноября\n\
             - Позвонить\nцитата\n\nСм. документ и код.\nПроблема, Срок\nОтчёты, пятница\n\
             snake_case stays"
        );
    }
}
