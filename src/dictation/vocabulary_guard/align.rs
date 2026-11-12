//! Words of a text as the guard compares them, and which words of the
//! transcript cleanup kept: the longest run of words both texts share, in
//! order, anchors the two; what lies between two anchors is a stretch
//! cleanup changed.

use std::collections::HashMap;

/// The largest alignment table. A piece of cleanup is some 400 words, so
/// only an answer many times its transcript passes this, and then nothing
/// is judged.
const MAX_CELLS: usize = 4_000_000;

/// One whitespace-separated word of a text.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Word<'a> {
    /// As written, with the punctuation around it.
    pub text: &'a str,
    /// Where `text` starts in the whole text, in bytes.
    pub start: usize,
    /// Without the punctuation around it; empty for bare punctuation.
    pub core: &'a str,
    /// `core` as compared: lowercase, "ё" as "е".
    pub norm: String,
}

impl Word<'_> {
    pub(super) fn end(&self) -> usize {
        self.start + self.text.len()
    }

    /// The punctuation after the word's core.
    pub(super) fn tail(&self) -> &str {
        let core_start = self.core.as_ptr() as usize - self.text.as_ptr() as usize;
        &self.text[core_start + self.core.len()..]
    }

    /// The punctuation before the word's core.
    pub(super) fn head(&self) -> &str {
        let core_start = self.core.as_ptr() as usize - self.text.as_ptr() as usize;
        &self.text[..core_start]
    }

    /// Whether a sentence ends with this word.
    pub(super) fn ends_sentence(&self) -> bool {
        self.tail().contains(['.', '!', '?', '…'])
    }
}

/// A word as compared: lowercase, without the punctuation around it.
pub(super) fn norm(core: &str) -> String {
    core.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .collect()
}

pub(super) fn words(text: &str) -> Vec<Word<'_>> {
    let mut words = Vec::new();
    let mut rest = text;
    loop {
        let trimmed = rest.trim_start();
        if trimmed.is_empty() {
            return words;
        }
        let length = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let token = &trimmed[..length];
        let core = token.trim_matches(|c: char| !c.is_alphanumeric());
        words.push(Word {
            text: token,
            start: text.len() - trimmed.len(),
            core,
            norm: norm(core),
        });
        rest = &trimmed[length..];
    }
}

/// For each word of `cleaned`, the word of `heard` it is, where cleanup kept
/// it: their longest common subsequence. `None` when the two are too long to
/// line up.
pub(super) fn kept(heard: &[Word<'_>], cleaned: &[Word<'_>]) -> Option<Vec<Option<usize>>> {
    // Words as numbers, so the table compares integers.
    let mut ids = HashMap::new();
    let mut next = 0;
    let a = numbered(heard, &mut ids, &mut next);
    let b = numbered(cleaned, &mut ids, &mut next);
    let mut pairs = vec![None; b.len()];
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    for (index, pair) in pairs.iter_mut().enumerate().take(prefix) {
        *pair = Some(index);
    }
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    for index in 0..suffix {
        pairs[b.len() - 1 - index] = Some(a.len() - 1 - index);
    }
    let (a_mid, b_mid) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    if a_mid.is_empty() || b_mid.is_empty() {
        return Some(pairs);
    }
    let width = b_mid.len() + 1;
    if (a_mid.len() + 1).saturating_mul(width) > MAX_CELLS || a_mid.len().max(b_mid.len()) >= 65_000
    {
        return None;
    }
    let mut table = vec![0_u16; (a_mid.len() + 1) * width];
    for i in (0..a_mid.len()).rev() {
        for j in (0..b_mid.len()).rev() {
            table[i * width + j] = if a_mid[i] == b_mid[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < a_mid.len() && j < b_mid.len() {
        if a_mid[i] == b_mid[j] {
            pairs[prefix + j] = Some(prefix + i);
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    Some(pairs)
}

/// Each word as a number that is the same for the same norm. Bare
/// punctuation gets a number of its own, so it never anchors.
fn numbered<'a>(
    words: &'a [Word<'_>],
    ids: &mut HashMap<&'a str, u32>,
    next: &mut u32,
) -> Vec<u32> {
    words
        .iter()
        .map(|word| {
            *next += 1;
            if word.norm.is_empty() {
                *next
            } else {
                *ids.entry(word.norm.as_str()).or_insert(*next)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_keep_their_place_and_lose_their_punctuation() {
        let text = "Open «Codex», then\n— wait.";
        let words = words(text);
        let cores: Vec<&str> = words.iter().map(|word| word.core).collect();
        assert_eq!(cores, ["Open", "Codex", "then", "", "wait"]);
        assert_eq!(words[1].norm, "codex");
        assert_eq!((words[1].head(), words[1].tail()), ("«", "»,"));
        assert_eq!(&text[words[4].start..words[4].end()], "wait.");
        assert!(words[4].ends_sentence() && !words[1].ends_sentence());
    }

    #[test]
    fn kept_words_anchor_and_changed_ones_do_not() {
        let heard = words("ну открой кодекс и закрой его");
        let cleaned = words("Открой Codex и закрой его.");
        assert_eq!(
            kept(&heard, &cleaned).unwrap(),
            [Some(1), None, Some(3), Some(4), Some(5)]
        );
        // Bare punctuation is never an anchor.
        let heard = words("да — нет");
        let cleaned = words("нет — да");
        let kept = kept(&heard, &cleaned).unwrap();
        assert_eq!(kept.iter().flatten().count(), 1);
    }
}
