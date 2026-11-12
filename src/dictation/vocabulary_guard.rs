//! Keeps AI cleanup honest about Vocabulary terms. A small model told to
//! spell "NightOwl" right also writes it for "barn owl" and for "night
//! shift", so after every cleanup, whichever engine wrote it, the
//! transcript cleanup was given is lined up with what it returned, and a
//! term cleanup wrote stays only where what was heard supports it.
//!
//! [`Guard::spell`] runs first: every exact "sounds like" spelling becomes
//! its term in code (`spelling`), so those never depend on the model.
//! [`Guard::check`] then judges each stretch cleanup changed that has a
//! term in it (`stretch`):
//!
//! - the heard words the term replaced must sound like the term or one of
//!   its spellings (`sound`), every word of it, across alphabets and with a
//!   Russian case ending allowed;
//! - a term nothing heard supports is undone: the stretch gets its heard
//!   words back (a term from nothing is removed), and the rest of what
//!   cleanup did stays;
//! - a heard word cleanup dropped beside a term, that isn't a filler, a
//!   repeat, or part of the term, is put back.
//!
//! When in doubt it keeps what cleanup wrote: a stretch rewritten too
//! freely to line up is left alone. No model is asked and nothing is
//! logged but counts.

mod align;
mod mend;
mod plain_words;
#[cfg(test)]
mod replay;
mod sound;
mod spelling;
mod stretch;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::ops::Range;

use crate::dictation::vocabulary::VocabularyEntry;
use Verdict::{Kept, Moved, Restored, Undone, Unjudged};
use align::{Word, words};
use sound::Key;
use stretch::{Gap, Stretch};

/// How far each side of an unsupported term its heard words are looked for
/// among the words cleanup dropped elsewhere, in case cleanup moved it.
const MOVED_WINDOW: usize = 40;
/// The most words cleanup may add of its own in a stretch that is undone:
/// past that it rewrote the stretch, and its words are not the guard's to
/// take back.
const MAX_STRAY_WORDS: usize = 2;

/// The Vocabulary as the guard reads it. Built once per dictation.
pub(crate) struct Guard {
    terms: Vec<Term>,
    /// A term's first word, as compared, to the terms starting with it,
    /// longest first.
    first_words: HashMap<String, Vec<usize>>,
    spellings: spelling::Spellings,
}

struct Term {
    /// As the list writes it.
    text: String,
    /// Its letters and digits, lowercase.
    letters: String,
    /// Its words as compared.
    words: Vec<String>,
    /// The term and each of its spellings as sounds.
    forms: Vec<Key>,
    /// Each word of each form as sounds.
    pieces: Vec<Key>,
    /// The shorter terms whose words this term has in a row ("cleanup" in
    /// "AI cleanup"), longest first, each with where its words start.
    inner: Vec<(usize, usize)>,
}

/// What [`Guard::check`] made of a cleanup.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Checked {
    pub text: String,
    /// Terms cleanup wrote that what was heard supports.
    pub kept: usize,
    /// Terms cleanup wrote that were undone.
    pub undone: usize,
    /// Heard words cleanup dropped beside a term that were put back.
    pub restored: usize,
    /// Terms in stretches rewritten too freely to judge, left as written.
    pub unjudged: usize,
    #[cfg(test)]
    pub notes: Vec<Note>,
}

/// What the guard did over one dictation's pieces, for the log: counts
/// only, never a word of what was said.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Tally {
    kept: usize,
    undone: usize,
    restored: usize,
    unjudged: usize,
}

impl Tally {
    /// Checks one piece's answer and returns the text that stands.
    pub(crate) fn check(&mut self, guard: &Guard, heard: &str, cleaned: String) -> String {
        if guard.terms.is_empty() {
            return cleaned;
        }
        let checked = guard.check(heard, &cleaned);
        self.kept += checked.kept;
        self.undone += checked.undone;
        self.restored += checked.restored;
        self.unjudged += checked.unjudged;
        checked.text
    }

    pub(crate) fn log(&self, profile: &str) {
        if self.undone + self.restored > 0 {
            tracing::info!(
                profile,
                terms_undone = self.undone,
                words_restored = self.restored,
                terms_kept = self.kept,
                terms_unjudged = self.unjudged,
                "vocabulary guard changed the cleanup"
            );
        }
    }
}

/// One judgement, for tests and the local replay only: production counts.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Note {
    pub term: String,
    /// The heard words of the stretch the term is in.
    pub heard: String,
    pub wrote: String,
    /// What stands there after the guard, where it changed the stretch.
    pub now: Option<String>,
    pub verdict: Verdict,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Verdict {
    /// What was heard supports the term.
    Kept,
    /// Its heard words were dropped elsewhere: cleanup moved it.
    Moved,
    /// Nothing heard supports it; the heard words are back.
    Undone,
    /// A heard word dropped beside it was put back.
    Restored,
    /// Its stretch was rewritten too freely to judge.
    Unjudged,
}

/// A term in the cleaned text: words `from..to` of it.
#[derive(Clone, Debug)]
pub(super) struct Mention {
    pub term: usize,
    pub from: usize,
    pub to: usize,
    /// Written as half of a hyphenated word ("MCP-сервер"), whose other
    /// half has heard words of its own: never undone.
    pub joined: bool,
}

impl Guard {
    pub(crate) fn new(vocabulary: &[VocabularyEntry]) -> Self {
        let mut guard = Self {
            terms: Vec::new(),
            first_words: HashMap::new(),
            spellings: spelling::Spellings::default(),
        };
        for entry in vocabulary {
            let text = entry.term.trim();
            let term_words: Vec<String> = words(text)
                .into_iter()
                .map(|word| word.norm)
                .filter(|norm| !norm.is_empty())
                .collect();
            if term_words.is_empty() {
                continue;
            }
            let index = guard.terms.len();
            let mut forms: Vec<Key> = Vec::new();
            let mut pieces: Vec<Key> = Vec::new();
            for form in std::iter::once(text).chain(entry.sounds_like.iter().map(String::as_str)) {
                let form_words = words(form);
                let key = Key::of(form_words.iter().map(|word| word.core), false);
                if !key.is_empty() && !forms.contains(&key) {
                    forms.push(key);
                }
                for word in &form_words {
                    let piece = Key::of([word.core], false);
                    if !piece.is_empty() && !pieces.contains(&piece) {
                        pieces.push(piece);
                    }
                }
            }
            guard
                .spellings
                .add(index, entry.sounds_like.iter().map(String::as_str));
            guard
                .first_words
                .entry(term_words[0].clone())
                .or_default()
                .push(index);
            guard.terms.push(Term {
                text: text.to_owned(),
                letters: spelling::letters(text),
                words: term_words,
                forms,
                pieces,
                inner: Vec::new(),
            });
        }
        for index in 0..guard.terms.len() {
            let own = &guard.terms[index].words;
            let mut inner: Vec<(usize, usize)> = guard
                .terms
                .iter()
                .enumerate()
                .filter(|(_, other)| other.words.len() < own.len())
                .filter_map(|(other, term)| {
                    let offset = own
                        .windows(term.words.len())
                        .position(|window| window == term.words)?;
                    Some((other, offset))
                })
                .collect();
            inner.sort_by_key(|&(other, _)| std::cmp::Reverse(guard.terms[other].words.len()));
            guard.terms[index].inner = inner;
        }
        let terms = &guard.terms;
        for starting in guard.first_words.values_mut() {
            starting.sort_by_key(|&term| std::cmp::Reverse(terms[term].words.len()));
        }
        guard
    }

    /// The transcript with every exact "sounds like" spelling written as
    /// its term (`spelling`), ahead of cleanup.
    pub(crate) fn spell(&self, transcript: &str) -> String {
        self.spellings
            .apply(
                transcript,
                |term| self.terms[term].text.as_str(),
                |term| self.terms[term].letters.as_str(),
            )
            .unwrap_or_else(|| transcript.to_owned())
    }

    /// `cleaned` (cleanup's answer for `heard`) with every term the
    /// transcript doesn't support undone.
    pub(crate) fn check(&self, heard: &str, cleaned: &str) -> Checked {
        let mut checked = Checked::default();
        if let Some(text) = self.repaired(heard, cleaned, &mut checked) {
            checked.text = text;
        } else {
            checked.text = cleaned.to_owned();
        }
        checked
    }

    fn repaired(
        &self,
        heard_text: &str,
        cleaned_text: &str,
        checked: &mut Checked,
    ) -> Option<String> {
        if self.terms.is_empty() {
            return None;
        }
        let cleaned = words(cleaned_text);
        let mentions = self.mentions(&cleaned);
        if mentions.is_empty() {
            return None;
        }
        let heard = words(heard_text);
        let mut kept = align::kept(&heard, &cleaned)?;
        let mut heard_kept = vec![false; heard.len()];
        for &index in kept.iter().flatten() {
            heard_kept[index] = true;
        }
        // A term the transcript already had, with nothing dropped beside
        // it, is settled. Every other term joins the stretch around it.
        let mut open: Vec<Mention> = Vec::new();
        for mention in mentions {
            let places = &kept[mention.from..mention.to];
            let settled = places[0].is_some_and(|first| {
                places
                    .iter()
                    .enumerate()
                    .all(|(offset, place)| *place == Some(first + offset))
                    && (first == 0 || heard_kept[first - 1])
                    && heard_kept
                        .get(first + places.len())
                        .copied()
                        .unwrap_or(true)
            });
            if settled {
                continue;
            }
            for place in &mut kept[mention.from..mention.to] {
                if let Some(index) = place.take() {
                    heard_kept[index] = false;
                }
            }
            open.push(mention);
        }
        if open.is_empty() {
            return None;
        }
        let mut gaps: Vec<Gap> = Vec::new();
        let mut scratch = stretch::Scratch::default();
        for stretch in stretches(&kept, heard.len(), &open) {
            match stretch.line_up(self, &heard, &cleaned, &open, &mut scratch) {
                Some(found) => gaps.extend(found),
                None => {
                    for mention in &open[stretch.mentions.clone()] {
                        checked.unjudged += 1;
                        let said = &heard[stretch.heard.clone()];
                        self.note(checked, mention, said, &cleaned, Unjudged, None);
                    }
                }
            }
        }
        self.find_moved(&mut gaps, &heard, &heard_kept, &open, &mut scratch);
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for gap in &gaps {
            if let Some(edit) = self.settle(gap, &heard, &cleaned, &open, checked, &mut scratch) {
                edits.push(edit);
            }
        }
        if edits.is_empty() {
            return None;
        }
        edits.sort_by_key(|(range, _)| range.start);
        let mut text = String::with_capacity(cleaned_text.len() + 32);
        let mut copied = 0;
        for (range, replacement) in edits {
            text.push_str(&cleaned_text[copied..range.start]);
            text.push_str(&replacement);
            copied = range.end;
        }
        text.push_str(&cleaned_text[copied..]);
        Some(text)
    }

    /// Every term in the cleaned text, the longest at each word, none
    /// overlapping. The last word may carry a Russian ending the model
    /// added ("Codex'ом").
    fn mentions(&self, cleaned: &[Word<'_>]) -> Vec<Mention> {
        let mut mentions = Vec::new();
        let mut at = 0;
        while at < cleaned.len() {
            let norm = cleaned[at].norm.as_str();
            let starting = self
                .first_words
                .get(norm)
                .or_else(|| self.first_words.get(uninflected(norm)?));
            let found = starting.and_then(|terms| {
                terms.iter().copied().find(|&term| {
                    let term_words = &self.terms[term].words;
                    let last = term_words.len() - 1;
                    at + term_words.len() <= cleaned.len()
                        && term_words.iter().enumerate().all(|(offset, word)| {
                            let norm = cleaned[at + offset].norm.as_str();
                            norm == word || (offset == last && uninflected(norm) == Some(word))
                        })
                })
            });
            match found {
                Some(term) => {
                    let to = at + self.terms[term].words.len();
                    let last = &cleaned[to - 1].norm;
                    let joined = last.contains('-') && !self.terms[term].words.contains(last);
                    mentions.push(Mention {
                        term,
                        from: at,
                        to,
                        joined,
                    });
                    at = to;
                }
                None => at += 1,
            }
        }
        mentions
    }

    /// Looks for the heard words of each unsupported term among the words
    /// cleanup dropped elsewhere: cleanup may have moved the term.
    fn find_moved(
        &self,
        gaps: &mut [Gap],
        heard: &[Word<'_>],
        heard_kept: &[bool],
        open: &[Mention],
        scratch: &mut stretch::Scratch,
    ) {
        let mut free: Vec<bool> = heard_kept.iter().map(|kept| !kept).collect();
        for gap in gaps.iter() {
            for placed in &gap.terms {
                if let Some(run) = &placed.run {
                    free[run.clone()].fill(false);
                }
            }
        }
        for index in 0..gaps.len() {
            let own = gaps[index].heard.clone();
            for placed in 0..gaps[index].terms.len() {
                if gaps[index].terms[placed].run.is_some() {
                    continue;
                }
                let term = &self.terms[open[gaps[index].terms[placed].mention].term];
                let from = own.start.saturating_sub(MOVED_WINDOW);
                let to = (own.end + MOVED_WINDOW).min(heard.len());
                let found = (from..to).find_map(|first| {
                    (1..=stretch::MAX_RUN_WORDS.min(to - first)).find_map(|count| {
                        let run = first..first + count;
                        let apart = run.end <= own.start || run.start >= own.end;
                        (apart
                            && free[run.clone()].iter().all(|free| *free)
                            && stretch::support(term, &heard[run.clone()], scratch).is_some())
                        .then_some(run)
                    })
                });
                if let Some(run) = found {
                    free[run].fill(false);
                    gaps[index].terms[placed].moved = true;
                }
            }
        }
    }

    /// Decides one gap and, where cleanup's words don't stand, what
    /// replaces them: bytes of the cleaned text and the new text.
    fn settle(
        &self,
        gap: &Gap,
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        open: &[Mention],
        checked: &mut Checked,
        scratch: &mut stretch::Scratch,
    ) -> Option<(Range<usize>, String)> {
        let heard_words = &heard[gap.heard.clone()];
        let unsupported = gap.terms.iter().any(|placed| !placed.supported());
        let joined = gap
            .terms
            .iter()
            .any(|placed| !placed.supported() && open[placed.mention].joined);
        if unsupported && (joined || gap.strays > MAX_STRAY_WORDS) {
            for placed in gap.terms.iter().filter(|placed| !placed.existing) {
                checked.unjudged += 1;
                self.note(
                    checked,
                    &open[placed.mention],
                    heard_words,
                    cleaned,
                    Unjudged,
                    None,
                );
            }
            return None;
        }
        if unsupported {
            let edit = self.undo(gap, heard, cleaned, open, scratch);
            for placed in gap.terms.iter().filter(|placed| !placed.existing) {
                let verdict = match (placed.supported(), placed.moved) {
                    (false, _) => {
                        checked.undone += 1;
                        Undone
                    }
                    (true, true) => {
                        checked.kept += 1;
                        Moved
                    }
                    (true, false) => {
                        checked.kept += 1;
                        Kept
                    }
                };
                let mention = &open[placed.mention];
                self.note(
                    checked,
                    mention,
                    heard_words,
                    cleaned,
                    verdict,
                    Some(&edit.1),
                );
            }
            return Some(edit);
        }
        for placed in gap.terms.iter().filter(|placed| !placed.existing) {
            checked.kept += 1;
            let verdict = if placed.moved { Moved } else { Kept };
            self.note(
                checked,
                &open[placed.mention],
                heard_words,
                cleaned,
                verdict,
                None,
            );
        }
        let swallowed = self.swallowed(gap, heard, cleaned, open, scratch);
        if swallowed.is_empty() || swallowed.len() <= gap.strays {
            return None;
        }
        checked.restored += swallowed.len();
        let edit = self.put_back(gap, &swallowed, heard, cleaned, open);
        if let Some(placed) = gap.terms.first() {
            let mention = &open[placed.mention];
            self.note(
                checked,
                mention,
                heard_words,
                cleaned,
                Restored,
                Some(&edit.1),
            );
        }
        Some(edit)
    }

    /// The heard words of a gap that cleanup dropped and shouldn't have:
    /// not a filler or a short word, and not a repeat or a part of a term
    /// beside them.
    fn swallowed(
        &self,
        gap: &Gap,
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        open: &[Mention],
        scratch: &mut stretch::Scratch,
    ) -> Vec<usize> {
        if gap.terms.iter().all(|placed| placed.run.is_none()) {
            return Vec::new();
        }
        gap.dropped
            .iter()
            .copied()
            .filter(|&index| {
                let word = &heard[index];
                // Kept as half of a hyphenated word ("MCP-сервер").
                let joined = cleaned[gap.cleaned.clone()]
                    .iter()
                    .any(|wrote| wrote.norm.split('-').any(|part| part == word.norm));
                plain_words::is_content(&word.norm)
                    && !joined
                    && !gap.terms.iter().any(|placed| {
                        stretch::part_of(&self.terms[open[placed.mention].term], word, scratch)
                    })
            })
            .collect()
    }

    #[cfg(test)]
    fn note(
        &self,
        checked: &mut Checked,
        mention: &Mention,
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        verdict: Verdict,
        now: Option<&str>,
    ) {
        let join = |words: &[Word<'_>]| {
            words
                .iter()
                .map(|word| word.text)
                .collect::<Vec<_>>()
                .join(" ")
        };
        checked.notes.push(Note {
            term: self.terms[mention.term].text.clone(),
            heard: join(heard),
            wrote: join(&cleaned[mention.from..mention.to]),
            now: now.map(str::to_owned),
            verdict,
        });
    }

    #[cfg(not(test))]
    #[allow(clippy::too_many_arguments)]
    fn note(
        &self,
        _: &mut Checked,
        _: &Mention,
        _: &[Word<'_>],
        _: &[Word<'_>],
        _: Verdict,
        _: Option<&str>,
    ) {
    }
}

/// The stretches of the cleaned text between kept words that have a term
/// to judge.
fn stretches(kept: &[Option<usize>], heard_len: usize, open: &[Mention]) -> Vec<Stretch> {
    let mut stretches = Vec::new();
    let mut mention = 0;
    let mut at = 0;
    let mut heard_from = 0;
    while at < kept.len() {
        if let Some(index) = kept[at] {
            heard_from = index + 1;
            at += 1;
            continue;
        }
        let from = at;
        while at < kept.len() && kept[at].is_none() {
            at += 1;
        }
        let heard_to = kept.get(at).copied().flatten().unwrap_or(heard_len);
        while mention < open.len() && open[mention].from < from {
            mention += 1;
        }
        let first = mention;
        while mention < open.len() && open[mention].from < at {
            mention += 1;
        }
        if mention > first {
            stretches.push(Stretch {
                heard: heard_from..heard_to.max(heard_from),
                cleaned: from..at,
                mentions: first..mention,
            });
        }
    }
    stretches
}

/// A cleaned word without what a model hangs on a Latin name: a Russian
/// ending ("codex'ом", "codex-а", "codexа") or a hyphenated word
/// ("mcp-сервер"); `None` when it has neither.
pub(super) fn uninflected(norm: &str) -> Option<&str> {
    if let Some((stem, rest)) = norm.split_once('-')
        && stem.ends_with(|c: char| c.is_ascii_alphanumeric())
        && rest.chars().all(|c| c.is_alphabetic() || c == '-')
    {
        return Some(stem);
    }
    let ending = norm
        .char_indices()
        .rev()
        .take_while(|(_, c)| ('а'..='я').contains(c))
        .last()?
        .0;
    let stem = norm[..ending].trim_end_matches(['\'', '’', 'ʼ', '`']);
    let short = norm[ending..].chars().count() <= 3;
    (short && stem.ends_with(|c: char| c.is_ascii_alphanumeric())).then_some(stem)
}
