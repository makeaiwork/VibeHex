//! How a term the use shows sounds, when neither a fix nor History says
//! (`recordings::sounds`): on a remote engine (Codex, OpenRouter, your own
//! server, OpenCode) the plan's model is told which terms have none and
//! may write 1 to 3 Russian spellings for each as `hex-vocab` lines, the
//! format it writes terms in. The app keeps only what passes [`valid`]:
//! letters of another alphabet than Latin, short, never the term itself,
//! at most [`MOST`]. The row stays the use's own (its reason, unchecked
//! unless sure); only its spellings come from the model.
//!
//! What the model reads is the terms' words alone, which `<since_last_setup>`
//! carries already, never a dictation's text. A built-in model writes no
//! plan (`flow`: the app's own on this device), so it is never asked: a
//! small model guesses a transliteration badly, and a wrong spelling, once
//! listed, is written as the term in code before cleanup
//! (`dictation::vocabulary_guard`'s exact spellings), rewriting what the
//! person said.

use super::since::Candidate;
use crate::assistant::changes::{Item, vocab};

/// The most spellings a term gets.
pub const MOST: usize = 3;
/// The most letters one spelling has.
const LETTERS: usize = 24;
/// The most words one spelling has.
const WORDS: usize = 3;

/// The terms with no spelling of how they are heard.
fn bare(terms: &[Candidate]) -> Vec<&str> {
    terms
        .iter()
        .filter(|term| term.sounds_like.is_empty())
        .map(|term| term.term.as_str())
        .collect()
}

/// What the plan's prompt adds for `terms`: nothing when every term has a
/// spelling already.
pub fn ask(terms: &[Candidate]) -> String {
    let bare = bare(terms);
    if bare.is_empty() {
        return String::new();
    }
    format!(
        "

<terms_without_spellings> lists terms the app will offer for Vocabulary with no spelling \
of how a Russian speech model writes them when they are said in Russian speech. For a term \
there whose Russian pronunciation you know, you may write one line in a ```hex-vocab block: \
{{\"term\":\"quit\",\"sounds_like\":[\"квит\"]}}, with the term exactly as listed and 1 to \
{MOST} short spellings in Russian letters, as heard, never the term itself. Only terms from \
that list; leave out one you don't know.

<terms_without_spellings>
{}
</terms_without_spellings>",
        bare.join("\n")
    )
}

/// `forms` a model wrote for `term`, as the app keeps them: each in
/// letters of another alphabet than Latin (a space or a hyphen between),
/// at most [`LETTERS`] letters and [`WORDS`] words, never the term itself,
/// each once, at most [`MOST`].
pub fn valid(term: &str, forms: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for form in forms {
        let form = form.split_whitespace().collect::<Vec<_>>().join(" ");
        let letters = form.chars().filter(|c| c.is_alphabetic()).count();
        let script = form
            .chars()
            .all(|c| (c.is_alphabetic() && !c.is_ascii()) || c == ' ' || c == '-');
        let fresh = !form.eq_ignore_ascii_case(term)
            && kept
                .iter()
                .all(|known| known.to_lowercase() != form.to_lowercase());
        if letters > 0
            && letters <= LETTERS
            && form.split(' ').count() <= WORDS
            && script
            && fresh
            && kept.len() < MOST
        {
            kept.push(form);
        }
    }
    kept
}

/// A model's term rows (`rows`) against the use's terms (`candidates`):
/// a row for a term the use has with no spelling gives that term its
/// valid spellings and goes; every other row stays as it was.
pub fn fill(rows: Vec<Item>, candidates: &mut [Candidate]) -> Vec<Item> {
    let mut left = Vec::new();
    for row in vocab::split(rows) {
        let terms = vocab::terms(&row.to);
        let [term] = terms.as_slice() else {
            left.push(row);
            continue;
        };
        let bare = candidates.iter_mut().find(|candidate| {
            candidate.sounds_like.is_empty() && candidate.term.eq_ignore_ascii_case(&term.term)
        });
        match bare {
            Some(candidate) => candidate.sounds_like = valid(&candidate.term, &term.sounds_like),
            None => left.push(row),
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forms(forms: &[&str]) -> Vec<String> {
        forms.iter().map(|form| (*form).to_owned()).collect()
    }

    #[test]
    fn only_short_spellings_in_another_alphabet_are_kept() {
        assert_eq!(
            valid("quit", &forms(&["квит", "  куит ", "quit", "Квит", "kvit"])),
            ["квит", "куит"]
        );
        // The term itself, Latin mixed in, too long, too many words.
        assert!(valid("e-mail", &forms(&["e-mail", "имейл1", "мейлmail"])).is_empty());
        assert!(valid("LEPS", &forms(&["лепслепслепслепслепслепслепс"])).is_empty());
        assert!(valid("Ask tool", &forms(&["аск тул вот это вот"])).is_empty());
        assert_eq!(
            valid("e-mail", &forms(&["имейл", "е-мейл", "мейл", "емейл"])),
            ["имейл", "е-мейл", "мейл"]
        );
    }

    #[test]
    fn the_prompt_lists_only_terms_with_no_spelling() {
        let terms = [
            Candidate {
                term: "quit".into(),
                ..Candidate::default()
            },
            Candidate {
                term: "GitHub".into(),
                sounds_like: vec!["гитхаб".into()],
                sure: true,
            },
        ];
        let asked = ask(&terms);
        assert!(asked.contains("<terms_without_spellings>\nquit\n</terms_without_spellings>"));
        assert!(!asked.contains("GitHub"));
        assert!(ask(&terms[1..]).is_empty());
    }

    /// A model's row for a bare term fills its spellings and goes; a row
    /// for a term that has some, or one the use doesn't have, stays.
    #[test]
    fn a_models_spellings_fill_the_bare_terms_only() {
        let row = |term: &str, sounds: &[&str]| Item {
            key: vocab::KEY.into(),
            to: vocab::Term {
                term: term.into(),
                sounds_like: forms(sounds),
            }
            .value(),
            ..Item::default()
        };
        let mut candidates = vec![
            Candidate {
                term: "LEPS".into(),
                ..Candidate::default()
            },
            Candidate {
                term: "GitHub".into(),
                sounds_like: vec!["гитхаб".into()],
                sure: true,
            },
        ];
        let left = fill(
            vec![
                row("leps", &["лепс", "LEPS", "лэпс"]),
                row("GitHub", &["гитхап"]),
                row("Kubernetes", &["кубернетис"]),
            ],
            &mut candidates,
        );
        assert_eq!(candidates[0].sounds_like, ["лепс", "лэпс"]);
        assert_eq!(candidates[1].sounds_like, ["гитхаб"]);
        let kept: Vec<String> = left
            .iter()
            .flat_map(|row| vocab::terms(&row.to))
            .map(|term| term.term)
            .collect();
        assert_eq!(kept, ["GitHub", "Kubernetes"]);
    }
}
