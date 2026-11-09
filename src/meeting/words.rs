//! Word times in a meeting's transcript: an entry keeps when each of its
//! words starts once the model timed them, so Listen to the recording can
//! light the word playing (Labs › Listen to the recording).

use super::{MeetingSource, TranscriptEntry};

/// `words` when it times each of `text`'s words, else none.
pub fn word_starts<'a>(text: &str, words: &'a [u64]) -> Option<&'a [u64]> {
    (!words.is_empty() && words.len() == text.split_whitespace().count()).then_some(words)
}

/// One sentence of timed words as an entry: its words joined by spaces,
/// each word's start kept (a word the model spaced inside, once per piece).
pub(super) fn sentence_entry(
    source: MeetingSource,
    group: &[crate::transcription::TranscriptSegment],
) -> TranscriptEntry {
    let start = |word: &crate::transcription::TranscriptSegment| word.start_ms.max(0) as u64;
    TranscriptEntry {
        source,
        start_ms: group.first().map_or(0, start),
        end_ms: group
            .iter()
            .map(|word| word.end_ms)
            .max()
            .unwrap_or_default() as u64,
        text: group
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        words: group
            .iter()
            .flat_map(|word| std::iter::repeat_n(start(word), word.text.split_whitespace().count()))
            .collect(),
    }
}
