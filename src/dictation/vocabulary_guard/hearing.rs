//! How a speech model writes a term it heard, found in a transcript by the
//! guard's own rule: a run of words in another alphabet than Latin that
//! [`support`] takes for the term (it sounds like it across alphabets,
//! every word of it, a Russian case ending allowed, a short word with every
//! consonant right), never the term itself, and no filler, particle, or
//! pronoun (`plain_words`). Setup reads it to say how a term Vocabulary
//! lacks is heard (`assistant::recordings::sounds`), so the forms it offers
//! are the ones this guard would hold against the term once it is listed.

use super::align::words;
use super::plain_words::is_plain;
use super::stretch::{MAX_RUN_WORDS, Scratch, support};
use super::{Guard, Term};
use crate::dictation::vocabulary::VocabularyEntry;

/// A run as heard needs this many letters: a word of one or two letters
/// ("ю", "ай") sounds like too many things.
const MIN_LETTERS: usize = 3;

/// One term, ready to be listened for.
pub(crate) struct Hearing {
    guard: Guard,
    /// The most heard words one run may have: the term's words and one
    /// more ("е мейл" for e-mail), at most the guard's own.
    longest: usize,
}

impl Hearing {
    /// `term` as the guard would read it from Vocabulary, with no
    /// spellings; `None` for a term with no words.
    pub(crate) fn of(term: &str) -> Option<Self> {
        let guard = Guard::new(&[VocabularyEntry {
            term: term.to_owned(),
            sounds_like: Vec::new(),
        }]);
        let words = guard.terms.first()?.words.len();
        Some(Self {
            guard,
            longest: (words + 1).min(MAX_RUN_WORDS),
        })
    }

    fn term(&self) -> &Term {
        &self.guard.terms[0]
    }

    /// Every run of `text` the guard takes for the term, as compared
    /// (lower case, "ё" as "е", the words joined by one space), best fit
    /// first where two runs start on the same word.
    pub(crate) fn find(&self, text: &str) -> Vec<String> {
        let words = words(text);
        let mut scratch = Scratch::default();
        let mut found = Vec::new();
        let mut at = 0;
        while at < words.len() {
            let mut best: Option<(f32, usize)> = None;
            for length in 1..=self.longest.min(words.len() - at) {
                let run = &words[at..at + length];
                if !run.iter().all(|word| heard_word(&word.norm)) {
                    break;
                }
                let letters: usize = run
                    .iter()
                    .map(|word| word.norm.chars().filter(|c| c.is_alphabetic()).count())
                    .sum();
                if letters < MIN_LETTERS {
                    continue;
                }
                if let Some((similarity, false)) = support(self.term(), run, &mut scratch)
                    && best.is_none_or(|(known, _)| similarity > known)
                {
                    best = Some((similarity, length));
                }
            }
            match best {
                Some((_, length)) => {
                    let run: Vec<&str> = words[at..at + length]
                        .iter()
                        .map(|word| word.norm.as_str())
                        .collect();
                    found.push(run.join(" "));
                    at += length;
                }
                None => at += 1,
            }
        }
        found
    }
}

/// `form` without the Russian case ending the guard leaves almost free on
/// what was heard ("лепсе" is "лепс", "имейлом" is "имейл"); as it is when
/// it has none.
pub(crate) fn stem(form: &str) -> String {
    let letters: Vec<char> = form.chars().collect();
    letters[..super::sound::stem_len(&letters)].iter().collect()
}

/// A word a speech model may have written for a term: letters of another
/// alphabet than Latin, and no filler, particle, or pronoun.
fn heard_word(norm: &str) -> bool {
    norm.chars().any(char::is_alphabetic)
        && norm
            .chars()
            .all(|c| (c.is_alphabetic() && !c.is_ascii()) || c == '-')
        && !is_plain(norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(term: &str, text: &str) -> Vec<String> {
        Hearing::of(term).unwrap().find(text)
    }

    #[test]
    fn a_term_written_in_russian_letters_is_found() {
        assert_eq!(found("LEPS", "открой лепс и посмотри"), ["лепс"]);
        assert_eq!(found("e-mail", "отправь на имейл сейчас"), ["имейл"]);
        assert_eq!(found("quit", "нажми квит потом"), ["квит"]);
        assert_eq!(found("Codex", "спроси кодексом"), ["кодексом"]);
        assert_eq!(found("Ask tool", "через аск тул спроси"), ["аск тул"]);
    }

    /// The term itself, a word that merely resembles it, a filler, and a
    /// word of one or two letters are never how it is heard.
    #[test]
    fn what_only_resembles_it_is_not() {
        assert!(found("LEPS", "LEPS и лепесток").is_empty());
        assert!(found("quit", "там кит и кот").is_empty());
        assert!(found("Codex", "код и кодировка").is_empty());
        assert!(found("UI", "ю и ну").is_empty());
        assert!(found("Docker", "кума это").is_empty());
    }

    #[test]
    fn a_stem_is_the_form_without_its_case_ending() {
        assert_eq!(stem("имейлом"), "имейл");
        assert_eq!(stem("лэбах"), "лэб");
        // A final "с" is no ending.
        assert_eq!(stem("лэпс"), "лэпс");
        assert_eq!(stem("кот"), "кот");
    }
}
