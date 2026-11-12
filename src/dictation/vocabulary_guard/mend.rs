//! What stands in a gap once the guard has decided against cleanup's
//! words: the heard words back where a term had no support (`undo`), or a
//! swallowed word back beside its term (`put_back`). Either way the answer
//! is the bytes of the cleaned text to replace and the text that replaces
//! them, so cleanup's own spacing and line breaks around the gap survive.

use std::collections::HashMap;
use std::ops::Range;

use super::align::Word;
use super::stretch::{self, Gap};
use super::{Guard, Mention};

impl Guard {
    /// The gap as heard, keeping the terms what was heard supports: the
    /// heard words in order, a supported term in place of its words. Where
    /// every term has heard words of its own (only a shorter term was
    /// heard), the fillers cleanup dropped stay dropped.
    pub(super) fn undo(
        &self,
        gap: &Gap,
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        open: &[Mention],
        scratch: &mut stretch::Scratch,
    ) -> (Range<usize>, String) {
        let as_heard = gap
            .terms
            .iter()
            .any(|placed| placed.run.is_none() && !placed.moved);
        let swallowed = if as_heard {
            Vec::new()
        } else {
            self.swallowed(gap, heard, cleaned, open, scratch)
        };
        let first_out = &cleaned[gap.cleaned.start];
        let last_out = &cleaned[gap.cleaned.end - 1];
        let starts_sentence = starts_sentence(cleaned, gap.cleaned.start);
        let mut tokens: Vec<String> = Vec::new();
        let mut index = gap.heard.start;
        // Terms cleanup moved here have no heard words in the gap: they
        // stay where cleanup put them, ahead of the gap's own words.
        for placed in gap.terms.iter().filter(|placed| placed.moved) {
            tokens.push(mention_text(cleaned, &open[placed.mention]));
        }
        while index < gap.heard.end {
            let run = gap
                .terms
                .iter()
                .find(|placed| placed.run.as_ref().is_some_and(|run| run.start == index));
            match run {
                Some(placed) => {
                    let mention = &open[placed.mention];
                    tokens.push(match &placed.shorter {
                        // Only the shorter term was heard: its words stay,
                        // with the punctuation that closed the longer one.
                        Some(shorter) => {
                            let mut text = cleaned[shorter.clone()]
                                .iter()
                                .map(|word| word.text)
                                .collect::<Vec<_>>()
                                .join(" ");
                            if shorter.end < mention.to && text.ends_with(char::is_alphanumeric) {
                                text.push_str(cleaned[mention.to - 1].tail());
                            }
                            text
                        }
                        None => mention_text(cleaned, mention),
                    });
                    index = placed.run.as_ref().map_or(index + 1, |run| run.end);
                }
                None if !as_heard && !swallowed.contains(&index) => index += 1,
                None => {
                    let word = &heard[index];
                    // A word capitalised only for starting the transcript's
                    // sentence loses that inside cleanup's sentence.
                    let lower = !(tokens.is_empty() && starts_sentence)
                        && starts_sentence_in(heard, index)
                        && word.core.chars().all(|c| !c.is_ascii_alphabetic());
                    tokens.push(if lower {
                        recased(word.text, false)
                    } else {
                        word.text.to_owned()
                    });
                    index += 1;
                }
            }
        }
        let from = first_out.start;
        let to = last_out.end();
        let Some(last) = tokens.last_mut() else {
            // Nothing was heard here: the term goes, with the space before
            // it; its closing punctuation moves to the word before.
            return match gap.cleaned.start.checked_sub(1) {
                Some(before) => {
                    let before = &cleaned[before];
                    let tail = if before.tail().is_empty() {
                        last_out.tail()
                    } else {
                        ""
                    };
                    (before.end()..to, tail.to_owned())
                }
                None => {
                    let next = cleaned.get(gap.cleaned.end).map_or(to, |next| next.start);
                    (from..next, String::new())
                }
            };
        };
        if last.ends_with(char::is_alphanumeric) {
            last.push_str(last_out.tail());
        }
        if starts_sentence && first_out.core.starts_with(char::is_uppercase) {
            tokens[0] = recased(&tokens[0], true);
        }
        if tokens[0].starts_with(char::is_alphanumeric) {
            tokens[0].insert_str(0, first_out.head());
        }
        (from..to, tokens.join(" "))
    }

    /// The gap as cleanup wrote it, with the swallowed heard words back
    /// beside the term they were dropped next to.
    pub(super) fn put_back(
        &self,
        gap: &Gap,
        swallowed: &[usize],
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        open: &[Mention],
    ) -> (Range<usize>, String) {
        // Each swallowed word goes before or after the nearest term that
        // has heard words of its own.
        let runs: Vec<(usize, Range<usize>)> = gap
            .terms
            .iter()
            .filter_map(|placed| Some((placed.mention, placed.run.clone()?)))
            .collect();
        let mut before: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut after: HashMap<usize, Vec<usize>> = HashMap::new();
        for &index in swallowed {
            let nearest = runs.iter().min_by_key(|(_, run)| {
                if index < run.start {
                    run.start - index
                } else {
                    index + 1 - run.end
                }
            });
            if let Some((mention, run)) = nearest {
                let side = if index < run.start {
                    &mut before
                } else {
                    &mut after
                };
                side.entry(*mention).or_default().push(index);
            }
        }
        let mut tokens: Vec<String> = Vec::new();
        let mut at = gap.cleaned.start;
        while at < gap.cleaned.end {
            let mention = gap
                .terms
                .iter()
                .map(|placed| placed.mention)
                .find(|&mention| open[mention].from == at);
            let Some(mention) = mention else {
                tokens.push(cleaned[at].text.to_owned());
                at += 1;
                continue;
            };
            let Mention { from, to, .. } = open[mention];
            let leads = before.get(&mention).map_or(&[][..], Vec::as_slice);
            let trails = after.get(&mention).map_or(&[][..], Vec::as_slice);
            let sentence = starts_sentence(cleaned, from);
            for (place, &index) in leads.iter().enumerate() {
                let lower = !(place == 0 && sentence)
                    && starts_sentence_in(heard, index)
                    && heard[index].core.chars().all(|c| !c.is_ascii_alphabetic());
                let core = heard[index].core;
                tokens.push(if place == 0 && sentence {
                    recased(core, true)
                } else if lower {
                    recased(core, false)
                } else {
                    core.to_owned()
                });
            }
            if trails.is_empty() {
                tokens.extend(cleaned[from..to].iter().map(|word| word.text.to_owned()));
            } else {
                // The term's closing punctuation closes the restored words.
                let last = &cleaned[to - 1];
                tokens.extend(
                    cleaned[from..to - 1]
                        .iter()
                        .map(|word| word.text.to_owned()),
                );
                tokens.push(format!("{}{}", last.head(), last.core));
                for &index in trails {
                    tokens.push(heard[index].core.to_owned());
                }
                if let Some(end) = tokens.last_mut() {
                    end.push_str(last.tail());
                }
            }
            at = to;
        }
        let from = cleaned[gap.cleaned.start].start;
        let to = cleaned[gap.cleaned.end - 1].end();
        (from..to, tokens.join(" "))
    }
}

fn mention_text(cleaned: &[Word<'_>], mention: &Mention) -> String {
    cleaned[mention.from..mention.to]
        .iter()
        .map(|word| word.text)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether the cleaned text's word `at` starts a sentence.
fn starts_sentence(cleaned: &[Word<'_>], at: usize) -> bool {
    at == 0 || cleaned[at - 1].ends_sentence()
}

fn starts_sentence_in(heard: &[Word<'_>], at: usize) -> bool {
    at == 0 || heard[at - 1].ends_sentence()
}

/// `token` with its first letter in upper or lower case.
fn recased(token: &str, upper: bool) -> String {
    let Some((at, first)) = token.char_indices().find(|(_, c)| c.is_alphabetic()) else {
        return token.to_owned();
    };
    let mut recased = String::with_capacity(token.len());
    recased.push_str(&token[..at]);
    if upper {
        recased.extend(first.to_uppercase());
    } else {
        recased.extend(first.to_lowercase());
    }
    recased.push_str(&token[at + first.len_utf8()..]);
    recased
}
