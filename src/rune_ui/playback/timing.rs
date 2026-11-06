//! When each word is heard. Transcripts keep times per line (a file's
//! segment, a meeting's turn), not per word, so a line's words share its
//! time by their length; a Cleanup's words take the times of the
//! Original's words they match, in order.

use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::files::store::Segment;

/// A word: its bytes in its text and when it is heard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Word {
    pub(super) range: Range<usize>,
    pub(super) at_ms: u64,
}

/// The words of `text[range]`, spread over `start_ms..end_ms` by length.
pub(super) fn spread(text: &str, range: Range<usize>, (start_ms, end_ms): (u64, u64)) -> Vec<Word> {
    let line = &text[range.clone()];
    let words: Vec<(usize, &str)> = line
        .split_whitespace()
        .map(|word| (word.as_ptr() as usize - line.as_ptr() as usize, word))
        .collect();
    let weight = |word: &str| word.chars().count() as u64 + 1;
    let total = words
        .iter()
        .map(|(_, word)| weight(word))
        .sum::<u64>()
        .max(1);
    let span = end_ms.saturating_sub(start_ms);
    let mut before = 0;
    words
        .into_iter()
        .map(|(offset, word)| {
            let at_ms = start_ms + span * before / total;
            before += weight(word);
            let start = range.start + offset;
            Word {
                range: start..start + word.len(),
                at_ms,
            }
        })
        .collect()
}

/// The word heard at `at_ms`: the last one started by then, else the first.
pub(super) fn heard_at(words: &[Word], at_ms: u64) -> Option<&Word> {
    words
        .iter()
        .take_while(|word| word.at_ms <= at_ms)
        .last()
        .or(words.first())
}

/// A word as matching compares it: lower case, letters and digits only.
fn plain(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .collect()
}

/// How far ahead in the Original a Cleanup word looks for itself.
const LOOK_AHEAD: usize = 24;

/// The time of each of `cleaned`'s words, from `heard`'s words (their plain
/// form and time) in order: a word found a step or two on takes its time,
/// one found further on only when the next word follows it there too, and
/// a word not found the time of the word before it.
fn align(heard: &[(String, u64)], cleaned: &[String]) -> Vec<u64> {
    let mut next = 0;
    let mut last = heard.first().map_or(0, |(_, at)| *at);
    let mut times = Vec::with_capacity(cleaned.len());
    for (index, word) in cleaned.iter().enumerate() {
        let found = (!word.is_empty())
            .then(|| {
                (next..heard.len().min(next + LOOK_AHEAD)).find(|&at| {
                    heard[at].0 == *word
                        && (at - next <= 2
                            || cleaned.get(index + 1).is_none_or(|following| {
                                heard.get(at + 1).is_some_and(|(then, _)| then == following)
                            }))
                })
            })
            .flatten();
        if let Some(at) = found {
            last = heard[at].1;
            next = at + 1;
        }
        times.push(last);
    }
    times
}

/// A Cleanup's paragraphs as shown, each with its words timed.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Cleaned {
    pub(super) paragraphs: Vec<(String, Vec<Word>)>,
}

/// `text`'s paragraphs, its words timed from `segments`' words.
fn clean_times(text: &str, segments: &[Segment]) -> Cleaned {
    let heard: Vec<(String, u64)> = segments
        .iter()
        .flat_map(|segment| {
            spread(
                &segment.text,
                0..segment.text.len(),
                (segment.start_ms, segment.end_ms),
            )
            .into_iter()
            .map(|word| (plain(&segment.text[word.range]), word.at_ms))
        })
        .filter(|(word, _)| !word.is_empty())
        .collect();
    let paragraphs: Vec<String> = text
        .split("\n\n")
        .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|paragraph| !paragraph.is_empty())
        .collect();
    let mut words: Vec<Vec<Word>> = paragraphs
        .iter()
        .map(|paragraph| spread(paragraph, 0..paragraph.len(), (0, 0)))
        .collect();
    let cleaned: Vec<String> = paragraphs
        .iter()
        .zip(&words)
        .flat_map(|(paragraph, words)| {
            words
                .iter()
                .map(|word| plain(&paragraph[word.range.clone()]))
        })
        .collect();
    let mut times = align(&heard, &cleaned).into_iter();
    for word in words.iter_mut().flatten() {
        word.at_ms = times.next().unwrap_or_default();
    }
    Cleaned {
        paragraphs: paragraphs.into_iter().zip(words).collect(),
    }
}

/// Text Markdown would draw differently from plain paragraphs.
pub(super) fn formatted(text: &str) -> bool {
    text.contains("**")
        || text.contains('`')
        || text.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with('#')
                || line.starts_with("- ")
                || line.starts_with("* ")
                || line.starts_with('>')
                || line.starts_with('|')
                || line.split_once(". ").is_some_and(|(number, _)| {
                    !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
                })
        })
}

/// The last Cleanup timed, so a frame doesn't align it again.
static LAST: Mutex<Option<(u64, Arc<Cleaned>)>> = Mutex::new(None);

/// `text`'s paragraphs, timed from `segments`, once per text.
pub(super) fn cleaned(text: &str, segments: &[Segment]) -> Arc<Cleaned> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    text.hash(&mut hasher);
    for segment in segments {
        (segment.start_ms, segment.end_ms, &segment.text).hash(&mut hasher);
    }
    let key = hasher.finish();
    let mut last = LAST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((kept, cleaned)) = last.as_ref()
        && *kept == key
    {
        return cleaned.clone();
    }
    let cleaned = Arc::new(clean_times(text, segments));
    *last = Some((key, cleaned.clone()));
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lines_words_share_its_time_by_length() {
        let text = "ab abcd ab";
        let words = spread(text, 0..text.len(), (1_000, 2_100));
        let starts: Vec<u64> = words.iter().map(|word| word.at_ms).collect();
        assert_eq!(starts, vec![1_000, 1_300, 1_800]);
        assert_eq!(&text[words[1].range.clone()], "abcd");
        assert_eq!(heard_at(&words, 1_500).map(|word| word.at_ms), Some(1_300));
        assert_eq!(heard_at(&words, 0).map(|word| word.at_ms), Some(1_000));
    }

    #[test]
    fn a_range_inside_a_paragraph_keeps_its_offset() {
        let text = "First. Then two words.";
        let words = spread(text, 7..text.len(), (0, 900));
        assert_eq!(&text[words[0].range.clone()], "Then");
        assert_eq!(&text[words[2].range.clone()], "words.");
    }

    #[test]
    fn cleanup_words_take_the_times_of_the_words_they_match() {
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "ну это самое".into(),
            },
            Segment {
                start_ms: 1_000,
                end_ms: 2_000,
                text: "Алена из Билайна".into(),
            },
        ];
        let cleaned = clean_times("Это — Алёна\n\nиз Билайна.", &segments);
        let times: Vec<Vec<u64>> = cleaned
            .paragraphs
            .iter()
            .map(|(_, words)| words.iter().map(|word| word.at_ms).collect())
            .collect();
        let said = |text: &str, times| spread(text, 0..text.len(), times);
        let heard = said("ну это самое", (0, 1_000));
        let alena = said("Алена из Билайна", (1_000, 2_000));
        assert_eq!(
            times,
            vec![
                vec![heard[1].at_ms, heard[1].at_ms, alena[0].at_ms],
                vec![alena[1].at_ms, alena[2].at_ms],
            ]
        );
    }

    #[test]
    fn a_common_word_far_ahead_needs_its_neighbour_to_match() {
        let heard: Vec<(String, u64)> = ["a", "b", "c", "d", "и", "e", "и", "f"]
            .iter()
            .enumerate()
            .map(|(index, word)| (word.to_string(), index as u64 * 100))
            .collect();
        let cleaned: Vec<String> = ["a", "и", "f"]
            .iter()
            .map(|word| word.to_string())
            .collect();
        assert_eq!(align(&heard, &cleaned), vec![0, 600, 700]);
    }

    #[test]
    fn markdown_keeps_its_own_drawing() {
        assert!(formatted("# Title\n\nText"));
        assert!(formatted("1. one\n2. two"));
        assert!(!formatted(
            "Plain words. Another paragraph.\n\nMore 2. words"
        ));
    }
}
