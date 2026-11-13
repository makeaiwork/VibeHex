//! Which Latin words in the person's dictations are terms Vocabulary
//! lacks. A term sits inside Russian speech (a Cyrillic word next to it,
//! never in an English sentence), is said often or keeps being fixed into,
//! and is not an ordinary English word (`english.txt`), unless it is
//! shaped like a name (`GitHub`, `API`, `gpt5`). A phrase cleanup keeps
//! writing ("pull request") is one term, as a whole. How a term was heard
//! comes only from fixes into exactly it, word for word or phrase for
//! phrase.
//!
//! An ordinary word written as a name (`Web`, `Monkey`), or a Title Case
//! phrase of ordinary words (`New Session`), is no term on its capital
//! alone ([`ordinary`]): cleanup writing «веб» as Web is a word put into
//! English, heard right. It is one only when it is misheard
//! (`heard::misheard`): a heard form that isn't the word itself, fixed
//! into it [`FIXED`] times or more, beside one form fixed [`SURE`] times.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use super::usage::{Fix, Term};

/// A term one heard word was fixed into this often is a sure one: setup
/// offers it checked.
pub const SURE: usize = 3;
/// Said inside Russian speech this often, a word that isn't ordinary
/// English is a term.
const SAID: usize = 2;
/// Fixed into this often, any word or phrase is a term.
const FIXED: usize = 2;
/// The most terms kept, and the most ways heard each keeps.
const MOST: usize = 15;
const HEARD: usize = 5;

/// Small words a phrase never starts or ends with: "to off to" is a
/// misheard run, not a term.
const SMALL: [&str; 16] = [
    "a", "an", "and", "as", "at", "by", "for", "from", "in", "is", "it", "of", "on", "or", "the",
    "to",
];

fn english() -> &'static HashSet<&'static str> {
    static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        include_str!("english.txt")
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .flat_map(str::split_whitespace)
            .collect()
    })
}

/// Whether `word` is an ordinary English word, or one's plural or -ed or
/// -ing form.
pub fn common(word: &str) -> bool {
    let lower = word.to_lowercase();
    let lower = lower.trim_end_matches("'s");
    let words = english();
    if words.contains(lower) {
        return true;
    }
    // `built-in`: each of its parts.
    if lower.contains('-') {
        return lower.split('-').all(|part| part.is_empty() || common(part));
    }
    let stem = |suffix: &str, then: &str| {
        lower
            .strip_suffix(suffix)
            .filter(|stem| stem.len() >= 2)
            .is_some_and(|stem| words.contains(format!("{stem}{then}").as_str()))
    };
    stem("ies", "y")
        || stem("es", "")
        || stem("s", "")
        || stem("ed", "")
        || stem("d", "")
        || stem("ing", "")
        || stem("ing", "e")
        || stem("ings", "")
        || stem("ings", "e")
}

/// Whether `word` is shaped like a name: a capital inside it (`GitHub`,
/// `macOS`), digits among its letters (`gpt5`), or all capitals that
/// aren't an ordinary word shouted (`API`, not `OK`).
pub fn brand(word: &str) -> bool {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    let capitals = letters.iter().filter(|c| c.is_uppercase()).count();
    if letters.len() >= 2 && capitals == letters.len() {
        return !common(word);
    }
    word.chars().any(|c| c.is_ascii_digit()) || word.chars().skip(1).any(|c| c.is_ascii_uppercase())
}

/// Whether `term` is ordinary English that only its capital makes a name:
/// one ordinary word (`Web`, `monkey`), or a phrase of ordinary words
/// written in Title Case (`New Session`; "pull request" is a term of the
/// trade and stays one).
pub fn ordinary(term: &str) -> bool {
    let words: Vec<&str> = term.split_whitespace().collect();
    let plain = |word: &&str| latin(word) && common(word) && !brand(word);
    let capital = |word: &&str| word.chars().next().is_some_and(char::is_uppercase);
    match words.as_slice() {
        [] => false,
        [one] => plain(one),
        many => {
            many.iter().all(plain)
                && many
                    .iter()
                    .all(|word| capital(word) || SMALL.contains(&word.to_lowercase().as_str()))
        }
    }
}

/// Whether a term a model or a follow-up names, with how it says it
/// sounds, passes the rule the use's terms pass: an ordinary word or
/// Title Case phrase needs a spelling that isn't the word itself.
pub fn passes(term: &str, sounds_like: &[String]) -> bool {
    !ordinary(term)
        || super::heard::misheard(term, sounds_like.iter().map(|heard| (heard.as_str(), 1))) > 0
}

/// Whether `word` is written in Latin letters: two or more, nothing but
/// ASCII (`Node.js`, `C++` keep their marks).
pub fn latin(word: &str) -> bool {
    word.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 2
        && word
            .chars()
            .all(|c| c.is_ascii() && !c.is_ascii_whitespace())
}

fn cyrillic(word: &str) -> bool {
    word.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
}

/// A Latin word said inside Russian speech: how often, in which spellings.
#[derive(Debug, Default)]
pub struct Said {
    count: usize,
    forms: HashMap<String, usize>,
}

/// Every Latin word said inside Russian speech, by its lower case.
pub type Seen = HashMap<String, Said>;

/// Notes the Latin words of one dictation's `words` with a Cyrillic word
/// beside them.
pub fn note(seen: &mut Seen, words: &[&str]) {
    for (at, word) in words.iter().enumerate() {
        if !latin(word) {
            continue;
        }
        let before = at
            .checked_sub(1)
            .is_some_and(|before| cyrillic(words[before]));
        let after = words.get(at + 1).is_some_and(|after| cyrillic(after));
        if !(before || after) {
            continue;
        }
        let said = seen.entry(word.to_lowercase()).or_default();
        said.count += 1;
        *said.forms.entry((*word).to_owned()).or_default() += 1;
    }
}

/// What Vocabulary has, in lower case: its terms, how they sound, and
/// each word of the ones made of several (`Code` of "Claude Code").
pub(super) fn knowns(known: &[String]) -> HashSet<String> {
    let mut all = HashSet::new();
    for known in known {
        let lower = known.trim().to_lowercase();
        for part in lower.split_whitespace() {
            all.insert(part.to_owned());
        }
        all.insert(lower);
    }
    all
}

/// The fixes into exactly `lower` (in any case), each a different heard
/// form: a single word from a single word, a phrase from a phrase.
fn into<'a>(fixes: &'a [Fix], lower: &str, phrase: bool) -> Vec<&'a Fix> {
    let mut into: Vec<&Fix> = fixes
        .iter()
        .filter(|fix| {
            fix.written.to_lowercase() == lower
                && fix.heard.to_lowercase() != lower
                && (phrase || !fix.heard.contains(' '))
        })
        .collect();
    into.sort_by(|a, b| b.count.cmp(&a.count).then(a.heard.cmp(&b.heard)));
    into
}

fn term(word: String, count: usize, into: &[&Fix]) -> Term {
    Term {
        word,
        count,
        heard: into
            .iter()
            .take(HEARD)
            .map(|fix| fix.heard.clone())
            .collect(),
        fixed: into.iter().map(|fix| fix.count).max().unwrap_or(0),
        sounds: Vec::new(),
    }
}

/// The terms the words `seen` and every fix cleanup made (`fixes`, each
/// counted) show, less what Vocabulary (`known`) has: sure ones first,
/// then the most said.
pub fn pick(seen: &Seen, fixes: &[Fix], known: &[String]) -> Vec<Term> {
    let known = knowns(known);
    let mut terms: Vec<Term> = Vec::new();
    for (lower, said) in seen {
        if known.contains(lower) {
            continue;
        }
        let Some(word) = said
            .forms
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(form, _)| form.clone())
        else {
            continue;
        };
        let into = into(fixes, lower, false);
        let fixed: usize = into.iter().map(|fix| fix.count).sum();
        let most = into.iter().map(|fix| fix.count).max().unwrap_or(0);
        let fits = if ordinary(&word) {
            // An ordinary word only when it keeps being misheard: one
            // heard word keeps becoming it, a form that isn't the word
            // itself among them («вольт» → Bolt), never a small one
            // ("то" → "to"), and written as a name every time. «веб» →
            // Web and тест → test are cleanup putting a word into
            // English, not terms.
            let named = said
                .forms
                .keys()
                .all(|form| form.chars().next().is_some_and(char::is_uppercase));
            let misheard = super::heard::misheard(
                &word,
                into.iter().map(|fix| (fix.heard.as_str(), fix.count)),
            );
            most >= SURE && misheard >= FIXED && named && !SMALL.contains(&lower.as_str())
        } else {
            said.count >= SAID || fixed >= FIXED
        };
        if !fits {
            continue;
        }
        terms.push(term(word, said.count, &into));
    }
    for (lower, (written, count)) in phrases(fixes) {
        if known.contains(&lower) || count < FIXED {
            continue;
        }
        let parts: Vec<&str> = lower.split(' ').collect();
        let edges = [parts[0], parts[parts.len() - 1]];
        if edges.iter().any(|edge| SMALL.contains(edge)) {
            continue;
        }
        // "New Session" for «нью сешн» is heard right, not a term.
        let into = into(fixes, &lower, true);
        let heard = into.iter().map(|fix| (fix.heard.as_str(), fix.count));
        if ordinary(&written) && super::heard::misheard(&written, heard) < FIXED {
            continue;
        }
        terms.retain(|term| {
            !(term.count <= count
                && parts
                    .iter()
                    .any(|part| term.word.eq_ignore_ascii_case(part)))
        });
        terms.push(term(written, count, &into));
    }
    let sure = |term: &Term| term.fixed >= SURE;
    terms.sort_by(|a, b| {
        sure(b)
            .cmp(&sure(a))
            .then(b.count.cmp(&a.count))
            .then(a.word.cmp(&b.word))
    });
    terms.truncate(MOST);
    terms
}

/// The phrases of two or three Latin words cleanup wrote for Russian it
/// heard, by their lower case: as most often written, and how often.
fn phrases(fixes: &[Fix]) -> Vec<(String, (String, usize))> {
    let mut phrases: HashMap<String, (HashMap<&str, usize>, usize)> = HashMap::new();
    for fix in fixes {
        let parts: Vec<&str> = fix.written.split(' ').collect();
        if parts.len() < 2 || !parts.iter().all(|part| latin(part)) || !cyrillic(&fix.heard) {
            continue;
        }
        let slot = phrases.entry(fix.written.to_lowercase()).or_default();
        *slot.0.entry(fix.written.as_str()).or_default() += fix.count;
        slot.1 += fix.count;
    }
    let mut phrases: Vec<(String, (String, usize))> = phrases
        .into_iter()
        .filter_map(|(lower, (forms, count))| {
            let written = forms
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))?
                .0
                .to_owned();
            Some((lower, (written, count)))
        })
        .collect();
    phrases.sort();
    phrases
}

/// The name in `names` that `word` is a near miss of: not the same, but
/// with the same first and last letters and a consonant skeleton (the
/// first letter, then the rest without vowels or repeats) one letter apart
/// at most, or the same for a short one. `Outkit` is a near miss of
/// `Orukeet`; `Granola` of no `Granite`. A name is checked whole, without
/// spaces, and by each of its words; one that is ordinary English
/// (`Output`) is heard right and never checked. The nearest wins, and the
/// first of `names` among equals.
pub fn named<'a>(word: &str, names: &'a [String]) -> Option<&'a str> {
    let mut best: Option<(usize, &str)> = None;
    for name in names {
        let parts = name.split_whitespace();
        let whole = std::iter::once(name.as_str()).filter(|_| name.contains(' '));
        for candidate in whole.chain(parts).filter(|part| !common(part)) {
            if let Some(apart) = near(word, candidate)
                && best.is_none_or(|(known, _)| apart < known)
            {
                best = Some((apart, candidate));
            }
        }
    }
    best.map(|(_, name)| name)
}

/// How far apart `word` and `name` are when one is a near miss of the
/// other.
fn near(word: &str, name: &str) -> Option<usize> {
    let flat = |text: &str| -> Vec<char> {
        text.chars()
            .filter(|letter| letter.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let (one, other) = (flat(word), flat(name));
    if one == other || one.first() != other.first() || one.last() != other.last() {
        return None;
    }
    let (one, other) = (skeleton(&one), skeleton(&other));
    let allowed = usize::from(one.len().min(other.len()) >= 4);
    let apart = distance(&one, &other);
    (apart <= allowed).then_some(apart)
}

/// The first letter, then the rest without vowels, repeats collapsed.
fn skeleton(letters: &[char]) -> Vec<char> {
    let mut kept: Vec<char> = letters.first().copied().into_iter().collect();
    for &letter in letters.iter().skip(1) {
        if !"aeiouy".contains(letter) && kept.last() != Some(&letter) {
            kept.push(letter);
        }
    }
    kept
}

/// Levenshtein distance.
pub(super) fn distance(one: &[char], other: &[char]) -> usize {
    let mut before: Vec<usize> = (0..=other.len()).collect();
    for (row, a) in one.iter().enumerate() {
        let mut now = vec![row + 1];
        for (column, b) in other.iter().enumerate() {
            let swap = before[column] + usize::from(a != b);
            now.push(swap.min(before[column + 1] + 1).min(now[column] + 1));
        }
        before = now;
    }
    before[other.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_words_and_names_are_told_apart() {
        for word in [
            "to",
            "and",
            "use",
            "items",
            "call",
            "Play",
            "History",
            "summaries",
            "used",
        ] {
            assert!(common(word), "{word}");
        }
        for word in ["Codex", "Granola", "kubectl", "GitHub"] {
            assert!(!common(word), "{word}");
        }
        for word in ["GitHub", "API", "gpt5", "macOS", "n8n"] {
            assert!(brand(word), "{word}");
        }
        for word in ["OK", "Code", "Structure", "codex"] {
            assert!(!brand(word), "{word}");
        }
    }

    #[test]
    fn a_near_miss_of_a_known_name_is_that_name() {
        let names: Vec<String> = [
            "Orukeet",
            "Granite Speech 5.0",
            "Whisper large-v3-turbo",
            "Codex",
        ]
        .map(str::to_owned)
        .to_vec();
        assert_eq!(named("Outkit", &names), Some("Orukeet"));
        assert_eq!(named("Codecs", &names), None);
        for word in ["Granola", "Monkey", "Whisper", "Codex", "MacBook", "Web"] {
            assert_eq!(named(word, &names), None, "{word}");
        }
        assert_eq!(named("Wisper", &names), Some("Whisper"));
    }

    #[test]
    fn a_capital_alone_makes_no_term_of_an_ordinary_word() {
        for term in [
            "Web",
            "Monkey",
            "Hero",
            "prompt",
            "New Session",
            "Speech to Text",
        ] {
            assert!(ordinary(term), "{term}");
        }
        for term in [
            "Codex",
            "GitHub",
            "Claude Code",
            "pull request",
            "API",
            "New Codex Session",
            "",
        ] {
            assert!(!ordinary(term), "{term}");
        }
        let heard = |forms: &[&str]| -> Vec<String> { forms.iter().map(|&f| f.into()).collect() };
        // The word itself in Russian letters is no evidence; another word is.
        assert!(!passes("Web", &heard(&["веб"])));
        assert!(!passes("Web", &[]));
        assert!(!passes("New Session", &heard(&["нью сешн"])));
        assert!(passes("Bolt", &heard(&["вольт"])));
        assert!(passes("Web", &heard(&["веб", "вэп"])));
        // A name needs none.
        assert!(passes("Codex", &[]));
        assert!(passes("pull request", &heard(&["пул реквест"])));
    }

    /// What the use shows, as `pick` reads it: each word said among
    /// Russian words, and the fixes cleanup made.
    #[test]
    fn the_use_offers_a_misheard_word_and_never_one_heard_right() {
        let fix = |heard: &str, written: &str, count: usize| Fix {
            heard: heard.into(),
            written: written.into(),
            count,
        };
        let mut seen = Seen::default();
        for word in ["Web", "Monkey", "Bolt", "Codex", "Hero"] {
            for _ in 0..6 {
                note(&mut seen, &["открой", word, "сейчас"]);
            }
        }
        let fixes = [
            // Heard right, written in English: not terms.
            fix("веб", "Web", 9),
            fix("монки", "Monkey", 5),
            fix("хиру", "Hero", 4),
            fix("нью сешн", "New Session", 4),
            // Misheard: «вольт» is another word.
            fix("вольт", "Bolt", 3),
            fix("болт", "Bolt", 2),
            // A name, and a term of the trade.
            fix("кодекс", "Codex", 3),
            fix("пул реквест", "pull request", 3),
        ];
        let picked: Vec<(String, bool)> = pick(&seen, &fixes, &[])
            .into_iter()
            .map(|term| (term.word.clone(), term.fixed >= SURE))
            .collect();
        let has = |word: &str| picked.iter().find(|(known, _)| known == word);
        for gone in ["Web", "Monkey", "Hero", "New Session"] {
            assert!(has(gone).is_none(), "{gone} in {picked:?}");
        }
        for (kept, sure) in [("Bolt", true), ("Codex", true), ("pull request", true)] {
            assert_eq!(has(kept), Some(&(kept.to_owned(), sure)), "{picked:?}");
        }
    }
}
