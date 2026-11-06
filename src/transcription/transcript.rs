//! A transcript with segment and word timings, as every OS's models give it.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
    /// Word timings, when the model produces them.
    #[serde(default)]
    pub words: Vec<TranscriptSegment>,
}

/// Words joined from a model's timed tokens, for a model that times its
/// tokens but not its words (GigaAM): a token starting with `▁` or a space
/// starts a word, any other (a word's next piece, punctuation) joins the
/// word before it. Each token is its start, end, and text.
pub fn words_from_tokens<'a>(
    tokens: impl IntoIterator<Item = (i64, i64, &'a str)>,
) -> Vec<TranscriptSegment> {
    let mut words: Vec<TranscriptSegment> = Vec::new();
    // A marker alone (GigaAM's space) opens the word after it.
    let mut open = true;
    for (start_ms, end_ms, text) in tokens {
        let piece = text.trim_start_matches(['\u{2581}', ' ']);
        open |= piece.len() != text.len();
        if piece.trim().is_empty() {
            continue;
        }
        match words.last_mut() {
            Some(word) if !open => {
                word.text.push_str(piece);
                word.end_ms = word.end_ms.max(end_ms);
            }
            _ => words.push(TranscriptSegment {
                start_ms,
                end_ms,
                text: piece.to_owned(),
            }),
        }
        open = false;
    }
    words
}

#[cfg(test)]
mod tests {
    #[test]
    fn tokens_join_into_words_at_their_markers() {
        let words = super::words_from_tokens([
            (80, 120, "\u{2581}Здравствуйте"),
            (720, 760, "."),
            (1_040, 1_080, "\u{2581}М"),
            (1_080, 1_120, "е"),
            (1_160, 1_200, "ня"),
            (1_240, 1_260, "\u{2581}"),
            (1_280, 1_300, "зовут"),
        ]);
        let read: Vec<(i64, i64, &str)> = words
            .iter()
            .map(|word| (word.start_ms, word.end_ms, word.text.as_str()))
            .collect();
        assert_eq!(
            read,
            [
                (80, 760, "Здравствуйте."),
                (1_040, 1_200, "Меня"),
                (1_280, 1_300, "зовут")
            ]
        );
    }
}
