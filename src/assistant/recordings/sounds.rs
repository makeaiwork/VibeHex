//! How a term Vocabulary lacks sounds in the person's own dictations, for
//! a term cleanup never fixed into it: a speech model that writes "quit"
//! in Latin leaves no fix to learn "квит" from, yet other dictations have
//! it as heard, before cleanup. The raw texts are held against the term by
//! the vocabulary guard's own sound matching
//! (`dictation::vocabulary_guard::Hearing`), so a form offered is one the
//! guard would hold against the term once it is listed. Only the forms
//! leave this module, never a dictation's text; it runs inside the use's
//! reading (`usage`), with the same leave.

use std::collections::HashMap;

use super::usage::Term;
use crate::dictation::vocabulary_guard::{self, Hearing};
use crate::history::HistoryEntry;

/// The most forms a term gets.
pub const MOST: usize = 3;
/// A form is heard at least this many times.
pub const SEEN: usize = 2;

/// Fills `sounds` of each of `terms` that has no heard form from a fix,
/// from the raw text of `entries`: the forms heard [`SEEN`] times or more,
/// most often first, at most [`MOST`]. Never the term itself, and never a
/// form that is another word heard: one the guard takes for a word
/// Vocabulary has (`known`, terms and their spellings; the owner's «лэп»
/// is Labs, listed as «Лэпс», not LEPS), for another of `terms`, or for a
/// form cleanup fixed into one.
pub fn fill(terms: &mut [Term], entries: &[HistoryEntry], known: &[String]) {
    let mut others: Vec<(String, Hearing)> = Vec::new();
    let words = known
        .iter()
        .chain(terms.iter().flat_map(|term| term.heard.iter()))
        .chain(terms.iter().map(|term| &term.word));
    for word in words {
        let folded = fold(word);
        if others.iter().all(|(known, _)| *known != folded)
            && let Some(hearing) = Hearing::of(word)
        {
            others.push((folded, hearing));
        }
    }
    let mut taken: Vec<String> = Vec::new();
    for term in terms.iter_mut().filter(|term| term.heard.is_empty()) {
        let Some(hearing) = Hearing::of(&term.word) else {
            continue;
        };
        let mut counts: HashMap<String, usize> = HashMap::new();
        for entry in entries {
            let raw = match entry.raw_text.trim() {
                "" => entry.final_text.as_str(),
                raw => raw,
            };
            for form in hearing.find(raw) {
                *counts.entry(form).or_default() += 1;
            }
        }
        let word = fold(&term.word);
        let elsewhere = |form: &str| {
            others.iter().any(|(other, hearing)| {
                *other != word && (other == form || !hearing.find(form).is_empty())
            })
        };
        let mut forms: Vec<(String, usize)> = declined(counts)
            .into_iter()
            .filter(|(form, count)| {
                *count >= SEEN && *form != word && !taken.contains(form) && !elsewhere(form)
            })
            .collect();
        forms.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        forms.truncate(MOST);
        term.sounds = forms.into_iter().map(|(form, _)| form).collect();
        taken.extend(term.sounds.iter().cloned());
    }
}

/// The forms with a declined one counted with the form it declines: "в
/// лепсе" is "лепс" heard, and the guard leaves a Russian case ending on
/// what was heard almost free (`vocabulary_guard::stem`).
fn declined(counts: HashMap<String, usize>) -> Vec<(String, usize)> {
    let mut forms: Vec<(String, usize)> = counts.into_iter().collect();
    forms.sort_by(|a, b| (a.0.chars().count(), &a.0).cmp(&(b.0.chars().count(), &b.0)));
    // By stem: the shortest form heard, and how often any of them was.
    let mut merged: Vec<(String, String, usize)> = Vec::new();
    for (form, count) in forms {
        let stem = stem(&form);
        match merged.iter_mut().find(|(known, ..)| *known == stem) {
            Some((_, _, known)) => *known += count,
            None => merged.push((stem, form, count)),
        }
    }
    merged
        .into_iter()
        .map(|(_, form, count)| (form, count))
        .collect()
}

/// A form's last word without its case ending.
fn stem(form: &str) -> String {
    match form.rsplit_once(' ') {
        Some((rest, last)) => format!("{rest} {}", vocabulary_guard::stem(last)),
        None => vocabulary_guard::stem(form),
    }
}

/// As compared: lower case, "ё" as "е".
fn fold(word: &str) -> String {
    word.trim().to_lowercase().replace('ё', "е")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, raw: &str, written: &str) -> HistoryEntry {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "timestamp_ms": 1_000 * id,
            "kind": "dictation",
            "raw_text": raw,
            "final_text": written,
        }))
        .unwrap()
    }

    fn term(word: &str, heard: &[&str]) -> Term {
        Term {
            word: word.into(),
            count: 3,
            heard: heard.iter().map(|heard| (*heard).to_owned()).collect(),
            fixed: 0,
            sounds: Vec::new(),
        }
    }

    /// The owner's quit, e-mail and LEPS: written in Latin when cleanup
    /// saw them, so no fix says how they are heard; History's raw text
    /// does. A form heard once, the term itself, an ordinary word that
    /// merely resembles it, a form Vocabulary has, and one cleanup fixed
    /// into another term are left out; a term with a heard form keeps it.
    #[test]
    fn a_terms_sounds_come_from_what_was_heard_before_cleanup() {
        let entries = [
            entry(1, "нажми квит и выйди", "нажми quit и выйди"),
            entry(2, "сделай квит сейчас", "сделай квит сейчас"),
            entry(3, "а потом квиту скажи", "а потом quit скажи"),
            entry(4, "куит это", "quit это"),
            entry(5, "напиши на имейл", "напиши на e-mail"),
            entry(6, "пришли имейлом", "пришли e-mail"),
            entry(7, "емейл забыл", "емейл забыл"),
            entry(8, "в лепсе и лепс", "в LEPS и LEPS"),
            entry(9, "там кит и кот", "там кит и кот"),
            entry(10, "там кит плывёт", "там кит плывёт"),
            entry(11, "кодекс и кодекс", "Codex и Codex"),
            entry(12, "гитхаб гитхаб", "GitHub GitHub"),
        ];
        let mut terms = [
            term("quit", &[]),
            term("e-mail", &[]),
            term("LEPS", &[]),
            term("Codex", &["кодекс"]),
            term("GitHub", &[]),
        ];
        fill(&mut terms, &entries, &["гитхаб".to_owned()]);
        // A Vocabulary spelling it fits: another word heard.
        let mut labs = [term("LEPS", &[])];
        fill(&mut labs, &entries, &["Labs".to_owned(), "Лэпс".to_owned()]);
        assert!(labs[0].sounds.is_empty(), "{:?}", labs[0].sounds);
        assert_eq!(terms[0].sounds, ["квит"]);
        assert_eq!(terms[1].sounds, ["имейл"]);
        assert_eq!(terms[2].sounds, ["лепс"]);
        // Its heard form stays what it has; History adds nothing.
        assert!(terms[3].sounds.is_empty());
        // Vocabulary has it already.
        assert!(terms[4].sounds.is_empty());
    }

    /// A declined form counts with the one it declines; past that, the
    /// three heard most often, most often first.
    #[test]
    fn at_most_three_forms_most_often_first() {
        let mut entries = Vec::new();
        let heard = [
            ("лепс", 2),
            ("лепсе", 2),
            ("лэпс", 3),
            ("липс", 2),
            ("лепз", 2),
            ("лепсом", 1),
        ];
        for (id, (raw, times)) in heard.iter().enumerate() {
            for time in 0..*times {
                entries.push(entry((id * 10 + time) as u64, raw, raw));
            }
        }
        let mut terms = [term("LEPS", &[])];
        fill(&mut terms, &entries, &[]);
        assert_eq!(terms[0].sounds, ["лепс", "лэпс", "лепз"]);
    }
}
