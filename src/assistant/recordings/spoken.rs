//! What the recordings search looks for: the question's words, each with
//! the other ways a transcript may write it. Those are the word's spoken
//! forms (`intent::Intent::spoken`) and, from the person's Vocabulary, a
//! term with its "sounds like" spellings, whichever of them was asked, so
//! «кубер» finds Kubernetes and VibeHex finds «вайб хекс». A Cyrillic
//! form also counts with a case ending («вайб хексе»). The Vocabulary is
//! read here, on the answer's thread, and never reaches a model.

use super::super::intent::Intent;
use crate::dictation::vocabulary::VocabularyEntry;

/// Russian case endings a Cyrillic form may carry in a transcript.
const ENDINGS: &[&str] = &["а", "е", "у", "ом", "ы", "ов"];

/// The search's query and, for each of its words, the spellings that stand
/// for it (as the meeting search reads a vocabulary).
#[derive(Debug, Default)]
pub(super) struct Query {
    pub text: String,
    pub aliases: Vec<VocabularyEntry>,
}

pub(super) fn query(intent: &Intent, vocabulary: &[VocabularyEntry]) -> Query {
    let terms = canonical(&intent.terms, vocabulary);
    let mut aliases = Vec::new();
    for term in &terms {
        let mut forms: Vec<String> = Vec::new();
        if let Some((_, spoken)) = intent.spoken.iter().find(|(known, _)| known == term) {
            spoken.iter().for_each(|form| add(&mut forms, term, form));
        }
        for entry in vocabulary {
            let spellings = || std::iter::once(&entry.term).chain(&entry.sounds_like);
            if !spellings().any(|spelling| names(term, spelling)) {
                continue;
            }
            spellings().for_each(|spelling| add(&mut forms, term, spelling));
            if entry.term.split_whitespace().count() == 1 {
                super::super::intent::spoken_forms_of(&entry.term)
                    .iter()
                    .for_each(|form| add(&mut forms, term, form));
            }
        }
        let inflected: Vec<String> = forms
            .iter()
            .filter(|form| ends_in_consonant(form))
            .flat_map(|form| ENDINGS.iter().map(move |ending| format!("{form}{ending}")))
            .collect();
        inflected
            .iter()
            .for_each(|form| add(&mut forms, term, form));
        if !forms.is_empty() {
            aliases.push(VocabularyEntry {
                term: term.clone(),
                sounds_like: forms,
            });
        }
    }
    Query {
        text: terms.join(" "),
        aliases,
    }
}

/// `form` among `term`'s forms, lowercase, once, and never `term` itself.
fn add(forms: &mut Vec<String>, term: &str, form: &str) {
    let form = lower(form);
    if !form.is_empty() && form != term && !forms.contains(&form) {
        forms.push(form);
    }
}

/// `terms` with a run that spells one of a Vocabulary term's "sounds like"
/// forms («вайб», «хекс») put back as the term (`vibehex`), so the search
/// finds every way it is written.
fn canonical(terms: &[String], vocabulary: &[VocabularyEntry]) -> Vec<String> {
    let mut terms = terms.to_vec();
    for entry in vocabulary {
        if entry.term.split_whitespace().count() != 1 {
            continue;
        }
        for spelling in &entry.sounds_like {
            let words: Vec<String> = spelling.split_whitespace().map(lower).collect();
            if words.len() < 2 || words.len() > terms.len() {
                continue;
            }
            let at = (0..=terms.len() - words.len()).find(|&at| {
                words
                    .iter()
                    .zip(&terms[at..])
                    .all(|(word, term)| names(term, word))
            });
            if let Some(at) = at {
                terms.splice(at..at + words.len(), [lower(&entry.term)]);
            }
        }
    }
    let mut seen = Vec::new();
    terms.retain(|term| {
        let new = !seen.contains(term);
        seen.push(term.clone());
        new
    });
    terms
}

/// Whether the question's `term` (a Russian word may be cut to a stem)
/// is `spelling`.
fn names(term: &str, spelling: &str) -> bool {
    let spelling = lower(spelling);
    spelling == term
        || (term.chars().count() >= 4
            && spelling.starts_with(term)
            && spelling.chars().count() - term.chars().count() <= 2)
}

fn lower(text: &str) -> String {
    text.trim().to_lowercase().replace('ё', "е")
}

fn ends_in_consonant(form: &str) -> bool {
    form.chars()
        .last()
        .is_some_and(|c| ('а'..='я').contains(&c) && !"аеиоуыэюяйь".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::intent;

    fn now() -> u64 {
        crate::meeting::query::parse_time("2026-11-27", 0, false).unwrap()
    }

    fn entry(term: &str, sounds_like: &[&str]) -> VocabularyEntry {
        VocabularyEntry {
            term: term.into(),
            sounds_like: sounds_like.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn a_name_brings_its_spoken_forms_and_their_endings() {
        let intent = intent::read("Найди мои диктовки про VibeHex", now()).unwrap();
        let found = query(&intent, &[]);
        assert_eq!(found.text, "vibehex");
        let forms = &found.aliases[0].sounds_like;
        for form in [
            "vibe hex",
            "вайб хекс",
            "вайпхекс",
            "вайб хексе",
            "вайбхекса",
        ] {
            assert!(forms.contains(&form.to_owned()), "{form} in {forms:?}");
        }
    }

    #[test]
    fn the_vocabulary_works_both_ways() {
        let vocabulary = [
            entry("Kubernetes", &["кубернетис", "кубер"]),
            entry("VibeHex", &["вайб хекс"]),
        ];
        let intent = intent::read("Найди мои диктовки про кубер", now()).unwrap();
        let found = query(&intent, &vocabulary);
        assert_eq!(found.text, "кубер");
        let forms = &found.aliases[0].sounds_like;
        assert!(forms.contains(&"kubernetes".to_owned()), "{forms:?}");
        assert!(forms.contains(&"кубернетис".to_owned()), "{forms:?}");
        // Two words that spell a term search as the term.
        let intent = intent::read("Найди мои диктовки про вайб хекс", now()).unwrap();
        let found = query(&intent, &vocabulary);
        assert_eq!(found.text, "vibehex");
        assert!(
            found.aliases[0]
                .sounds_like
                .contains(&"вайпхекс".to_owned())
        );
    }
}
