//! The list's exact "sounds like" spellings, written as their term before
//! cleanup sees the transcript: the whole list, whatever the speech model
//! and however long the list, with no scoring.
//!
//! It follows the rules of the pass Parakeet's biasing runs after decoding
//! (transcribe.cpp's `force_vocabulary_spellings`), so a transcript that
//! pass already spelled is left as it is: a spelling matches whole words,
//! up to four of them, by its letters and digits alone (so "найт оул" also
//! matches "найтоул" and "Найт-оул"); the longest match wins, then the
//! earliest; a spelling under two characters is ignored; a word with
//! address punctuation in it ("opencode.ai", "a/b") is never part of a
//! match; and only the last word may carry punctuation after it. Unlike
//! that pass it ignores case in every alphabet, not only in ASCII.

use std::collections::HashMap;
use std::ops::Range;

use super::align::{Word, words};

/// The most words one spelling match spans.
const MAX_SPAN_WORDS: usize = 4;
/// The fewest characters a spelling needs to count.
const MIN_SPELLING_CHARS: usize = 2;

#[derive(Default)]
pub(super) struct Spellings {
    /// A spelling's letters to its term; the first term listed wins.
    terms: HashMap<String, usize>,
}

/// A text's letters and digits, lowercase: what a spelling is matched by.
pub(super) fn letters(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

impl Spellings {
    /// Lists `spellings` for the term numbered `term`.
    pub(super) fn add<'a>(&mut self, term: usize, spellings: impl IntoIterator<Item = &'a str>) {
        for spelling in spellings {
            let letters = letters(spelling);
            if letters.chars().count() >= MIN_SPELLING_CHARS {
                self.terms.entry(letters).or_insert(term);
            }
        }
    }

    /// `text` with every listed spelling written as its term, or `None`
    /// when it has none. `term_text` gives a term as written and
    /// `term_letters` its own letters.
    pub(super) fn apply<'t>(
        &self,
        text: &str,
        term_text: impl Fn(usize) -> &'t str,
        term_letters: impl Fn(usize) -> &'t str,
    ) -> Option<String> {
        let matches = self.matches(text, &term_text, term_letters);
        if matches.is_empty() {
            return None;
        }
        let mut spelled = String::with_capacity(text.len());
        let mut copied = 0;
        for (range, term) in matches {
            spelled.push_str(&text[copied..range.start]);
            spelled.push_str(term_text(term));
            copied = range.end;
        }
        spelled.push_str(&text[copied..]);
        Some(spelled)
    }

    /// Where `text` has a listed spelling, in order: its bytes and its term.
    pub(super) fn matches<'t>(
        &self,
        text: &str,
        term_text: impl Fn(usize) -> &'t str,
        term_letters: impl Fn(usize) -> &'t str,
    ) -> Vec<(Range<usize>, usize)> {
        if self.terms.is_empty() {
            return Vec::new();
        }
        let words = words(text);
        // (first word, word count, term)
        let mut found: Vec<(usize, usize, usize)> = Vec::new();
        let mut span = String::new();
        for first in 0..words.len() {
            span.clear();
            for count in 1..=MAX_SPAN_WORDS.min(words.len() - first) {
                let word = &words[first + count - 1];
                if !joins(word, count > 1) {
                    break;
                }
                span.push_str(&letters(word.core));
                if let Some(&term) = self.terms.get(span.as_str()) {
                    // One word that differs from the term only in case is
                    // not a mishearing, and the term as written needs
                    // nothing.
                    let case_only = count == 1 && span == term_letters(term);
                    let written = words[first..first + count]
                        .iter()
                        .map(|word| word.core)
                        .eq(term_text(term).split_whitespace());
                    if !case_only && !written {
                        found.push((first, count, term));
                    }
                }
                // Punctuation after a word ends the span there.
                if !word.tail().is_empty() {
                    break;
                }
            }
        }
        found.sort_by_key(|&(first, count, _)| (std::cmp::Reverse(count), first));
        let mut chosen: Vec<(usize, usize, usize)> = Vec::new();
        for (first, count, term) in found {
            if chosen
                .iter()
                .all(|&(other, length, _)| first + count <= other || other + length <= first)
            {
                chosen.push((first, count, term));
            }
        }
        chosen.sort_by_key(|&(first, ..)| first);
        chosen
            .into_iter()
            .map(|(first, count, term)| {
                let from = words[first].start + words[first].head().len();
                let last = &words[first + count - 1];
                let to = last.start + last.head().len() + last.core.len();
                (from..to, term)
            })
            .collect()
    }
}

/// Whether `word` can be part of a spelling match: it has letters, none of
/// an address's punctuation, and, past the match's first word, nothing
/// before its letters.
fn joins(word: &Word<'_>, later: bool) -> bool {
    !word.core.is_empty()
        && !word.core.contains(['.', '/', '@', ':', '\\', '_'])
        && !(later && !word.head().is_empty())
}
