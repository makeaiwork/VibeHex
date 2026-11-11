//! What the person's own use shows, for a returning setup: counts and
//! shares, the apps dictated into, the words cleanup keeps fixing and the
//! terms Vocabulary lacks, word by word. Computed here, by the app, and
//! only with [`Allowed`]; no dictation's or meeting's text leaves this
//! module, only these aggregates (`lines`).

use std::collections::HashMap;

use super::{Allowed, Sources};
use crate::history::HistoryEntry;
use crate::meeting::query::Library;

/// A dictation this many words long or longer is a long one.
pub const LONG_WORDS: usize = 60;
/// The most terms and fixes kept.
const TERMS: usize = 15;
const FIXES: usize = 10;
/// The most apps named.
const APPS: usize = 5;
/// A dictation's words compared at most, each side.
const DIFF_WORDS: usize = 400;
/// A fix is at most this many words either side.
const FIX_WORDS: usize = 3;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// Apps that are a terminal or a code editor, by a part of their name.
const CODE_APPS: [&str; 22] = [
    "terminal",
    "iterm",
    "warp",
    "ghostty",
    "kitty",
    "alacritty",
    "wezterm",
    "visual studio code",
    "vscode",
    "code",
    "cursor",
    "windsurf",
    "zed",
    "xcode",
    "intellij",
    "pycharm",
    "webstorm",
    "rustrover",
    "goland",
    "sublime",
    "neovim",
    "nova",
];

/// What the person's use shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub dictations: usize,
    pub last_30_days: usize,
    /// The apps dictated into most, with how many each.
    pub apps: Vec<(String, usize)>,
    /// Of the dictations with an app, the share into a terminal or a code
    /// editor, in percent.
    pub code_share: u32,
    /// The share of [`LONG_WORDS`]+ dictations, in percent.
    pub long_share: u32,
    pub median_words: usize,
    /// Latin-script terms said often (or fixed), not in Vocabulary.
    pub terms: Vec<Term>,
    /// What cleanup changed more than once, as heard and as written.
    pub fixes: Vec<Fix>,
    pub meetings: usize,
    /// The meetings by the language their transcript reads in.
    pub meeting_languages: Vec<(Language, usize)>,
}

/// A term Vocabulary lacks.
#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub word: String,
    pub count: usize,
    /// How the speech model heard it before cleanup fixed it.
    pub heard: Vec<String>,
}

/// One change cleanup made more than once.
#[derive(Clone, Debug, PartialEq)]
pub struct Fix {
    pub heard: String,
    pub written: String,
    pub count: usize,
}

/// The language a meeting's transcript reads in, by its letters.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Language {
    Russian,
    /// English, or another language in Latin letters.
    Latin,
}

impl Language {
    /// Its name, in English for `tr`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Russian => "Russian",
            Self::Latin => "English or another language",
        }
    }
}

/// The person's use, from History and the meetings, with `known` the
/// words Vocabulary has (terms and how they sound).
pub fn read(_allowed: &Allowed, sources: &Sources, known: &[String], now_ms: u64) -> Usage {
    let entries = crate::history::read_snapshot(&sources.history).unwrap_or_default();
    let mut usage = from_entries(&entries, known, now_ms);
    if let Ok(scan) = Library::new(&sources.meetings).scan() {
        let done = scan.meetings.iter().filter(|meeting| !meeting.in_progress);
        let mut languages: HashMap<Language, usize> = HashMap::new();
        for meeting in done {
            usage.meetings += 1;
            let text: String = meeting
                .entries
                .iter()
                .map(|entry| entry.text.as_str())
                .collect();
            if let Some(language) = language(&text) {
                *languages.entry(language).or_default() += 1;
            }
        }
        usage.meeting_languages = ranked(languages, usize::MAX);
    }
    usage
}

/// The dictations' part of [`Usage`].
pub fn from_entries(entries: &[HistoryEntry], known: &[String], now_ms: u64) -> Usage {
    let known: Vec<String> = known.iter().map(|word| word.to_lowercase()).collect();
    let mut usage = Usage {
        dictations: entries.len(),
        last_30_days: entries
            .iter()
            .filter(|entry| entry.timestamp_ms + 30 * DAY_MS >= now_ms)
            .count(),
        ..Usage::default()
    };
    let mut apps: HashMap<String, usize> = HashMap::new();
    let (mut with_app, mut code, mut long) = (0, 0, 0);
    let mut lengths = Vec::new();
    let mut terms: HashMap<String, (String, usize)> = HashMap::new();
    let mut fixes: HashMap<(String, String), usize> = HashMap::new();
    for entry in entries {
        if let Some(app) = entry.application.as_deref().filter(|app| !app.is_empty()) {
            with_app += 1;
            *apps.entry(app.to_owned()).or_default() += 1;
            let app = app.to_lowercase();
            if CODE_APPS.iter().any(|known| app.contains(known)) {
                code += 1;
            }
        }
        let written = words(&entry.final_text);
        lengths.push(written.len());
        if written.len() >= LONG_WORDS {
            long += 1;
        }
        // Latin words in Russian speech are terms.
        let cyrillic = entry.final_text.chars().any(is_cyrillic);
        for word in &written {
            if is_term(word, cyrillic) {
                let slot = terms
                    .entry(word.to_lowercase())
                    .or_insert_with(|| ((*word).to_owned(), 0));
                slot.1 += 1;
            }
        }
        for (heard, fixed) in changed(&words(&entry.raw_text), &written) {
            *fixes.entry((heard, fixed)).or_default() += 1;
        }
    }
    usage.apps = ranked(apps, APPS);
    usage.code_share = percent(code, with_app);
    usage.long_share = percent(long, entries.len());
    lengths.sort_unstable();
    usage.median_words = lengths.get(lengths.len() / 2).copied().unwrap_or(0);
    let mut fixes: Vec<Fix> = fixes
        .into_iter()
        .map(|((heard, written), count)| Fix {
            heard,
            written,
            count,
        })
        .collect();
    fixes.sort_by(|a, b| b.count.cmp(&a.count).then(a.heard.cmp(&b.heard)));
    let mut terms: Vec<Term> = terms
        .into_iter()
        .filter(|(lower, _)| !known.contains(lower))
        .map(|(lower, (word, count))| Term {
            heard: fixes
                .iter()
                .filter(|fix| {
                    fix.written
                        .split(' ')
                        .any(|part| part.to_lowercase() == lower)
                })
                .map(|fix| fix.heard.clone())
                .collect(),
            word,
            count,
        })
        .filter(|term| term.count >= 2 || !term.heard.is_empty())
        .collect();
    // A fix to a few Latin words is one term ("pull request"), not each.
    for fix in &fixes {
        let parts: Vec<&str> = fix.written.split(' ').collect();
        let latin = parts.iter().all(|part| is_term(part, true));
        let lower = fix.written.to_lowercase();
        if parts.len() < 2 || !latin || known.contains(&lower) {
            continue;
        }
        terms.retain(|term| {
            !(term.count <= fix.count
                && parts
                    .iter()
                    .any(|part| part.eq_ignore_ascii_case(&term.word)))
        });
        terms.push(Term {
            word: fix.written.clone(),
            count: fix.count,
            heard: vec![fix.heard.clone()],
        });
    }
    terms.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    terms.truncate(TERMS);
    fixes.retain(|fix| fix.count >= 2);
    fixes.truncate(FIXES);
    usage.terms = terms;
    usage.fixes = fixes;
    usage
}

/// The aggregates as the model reads them, in English: no text of any
/// dictation or meeting, only counts, names, and single words.
pub fn lines(usage: &Usage) -> Vec<String> {
    let mut lines = vec![format!(
        "dictations kept: {}, {} in the last 30 days",
        usage.dictations, usage.last_30_days
    )];
    if !usage.apps.is_empty() {
        let apps: Vec<String> = usage
            .apps
            .iter()
            .map(|(app, count)| format!("{app} {count}"))
            .collect();
        lines.push(format!("apps dictated into most: {}", apps.join(", ")));
        lines.push(format!(
            "into a terminal or code editor: {}%",
            usage.code_share
        ));
    }
    if usage.dictations > 0 {
        lines.push(format!(
            "long dictations ({LONG_WORDS}+ words): {}%, median {} words",
            usage.long_share, usage.median_words
        ));
    }
    if !usage.terms.is_empty() {
        let terms: Vec<String> = usage.terms.iter().map(term_line).collect();
        lines.push(format!("terms not in Vocabulary: {}", terms.join(", ")));
    }
    if !usage.fixes.is_empty() {
        let fixes: Vec<String> = usage
            .fixes
            .iter()
            .map(|fix| format!("\"{}\" -> \"{}\" x{}", fix.heard, fix.written, fix.count))
            .collect();
        lines.push(format!("cleanup keeps fixing: {}", fixes.join(", ")));
    }
    if usage.meetings > 0 {
        let languages: Vec<String> = usage
            .meeting_languages
            .iter()
            .map(|(language, count)| format!("{} {count}", language.name()))
            .collect();
        lines.push(format!(
            "meetings recorded: {} ({})",
            usage.meetings,
            languages.join(", ")
        ));
    }
    lines
}

fn term_line(term: &Term) -> String {
    if term.heard.is_empty() {
        format!("{} x{}", term.word, term.count)
    } else {
        format!(
            "{} x{} (heard as \"{}\")",
            term.word,
            term.count,
            term.heard.join("\", \"")
        )
    }
}

/// A text's words, their punctuation trimmed; `.`, `+` and `#` inside a
/// word stay (`Node.js`, `C++`, `C#`).
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| c.is_whitespace() || ",;:!?()[]{}«»\"“”".contains(c))
        .map(|word| word.trim_matches(|c: char| ".'’-—–…".contains(c)))
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .collect()
}

/// Whether `word` looks like a term: Latin letters inside Russian speech,
/// or anywhere with a capital inside it (`GitHub`), all capitals
/// (`API`), or digits among its letters (`gpt5`).
fn is_term(word: &str, cyrillic: bool) -> bool {
    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
    if letters < 2
        || !word
            .chars()
            .all(|c| c.is_ascii() && !c.is_ascii_whitespace())
    {
        return false;
    }
    let inner_capital = word.chars().skip(1).any(|c| c.is_ascii_uppercase());
    let digits = word.chars().any(|c| c.is_ascii_digit());
    cyrillic || inner_capital || digits
}

/// Russian when most of `text`'s letters are Cyrillic, else Latin; none
/// without letters.
fn language(text: &str) -> Option<Language> {
    let (mut cyrillic, mut latin) = (0usize, 0usize);
    for c in text.chars().filter(|c| c.is_alphabetic()) {
        if is_cyrillic(c) {
            cyrillic += 1;
        } else if c.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    match (cyrillic, latin) {
        (0, 0) => None,
        _ if cyrillic > latin => Some(Language::Russian),
        _ => Some(Language::Latin),
    }
}

fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}

/// What changed from `heard` to `written`, word by word: each run of up
/// to [`FIX_WORDS`] words replaced by another, lower case as heard.
fn changed(heard: &[&str], written: &[&str]) -> Vec<(String, String)> {
    if heard.len() > DIFF_WORDS || written.len() > DIFF_WORDS {
        return Vec::new();
    }
    let fold = |words: &[&str]| -> Vec<String> { words.iter().map(|w| w.to_lowercase()).collect() };
    let (a, b) = (fold(heard), fold(written));
    // The longest common subsequence, from the end.
    let mut table = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i][j] = if a[i] == b[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut runs = Vec::new();
    let (mut gone, mut came): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
    let mut flush = |gone: &mut Vec<&str>, came: &mut Vec<&str>| {
        if !gone.is_empty()
            && !came.is_empty()
            && gone.len() <= FIX_WORDS
            && came.len() <= FIX_WORDS
        {
            runs.push((gone.join(" ").to_lowercase(), came.join(" ")));
        }
        gone.clear();
        came.clear();
    };
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            flush(&mut gone, &mut came);
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || table[i][j + 1] >= table[i + 1][j]) {
            came.push(written[j]);
            j += 1;
        } else {
            gone.push(heard[i]);
            i += 1;
        }
    }
    flush(&mut gone, &mut came);
    runs
}

fn ranked<K: Ord>(counts: HashMap<K, usize>, most: usize) -> Vec<(K, usize)> {
    let mut ranked: Vec<(K, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(most);
    ranked
}

fn percent(part: usize, whole: usize) -> u32 {
    if whole == 0 {
        0
    } else {
        ((part * 100 + whole / 2) / whole) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::HistoryKind;

    fn fixture() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assistant/recordings")
    }

    fn entry(id: u64, app: &str, raw: &str, written: &str) -> HistoryEntry {
        let mut entry: HistoryEntry = serde_json::from_value(serde_json::json!({
            "id": id,
            "timestamp_ms": 1_000 * id,
            "kind": "dictation",
            "raw_text": raw,
            "final_text": written,
        }))
        .unwrap();
        entry.application = Some(app.into());
        assert_eq!(entry.kind, HistoryKind::Dictation);
        entry
    }

    /// The fixture's first five dictations: counts, apps, lengths, the one
    /// term said in Russian; its meetings and their languages.
    #[test]
    fn the_fixtures_use_adds_up() {
        let root = fixture();
        let mut entries = crate::history::read_snapshot(&root.join("history.json")).unwrap();
        entries.retain(|entry| entry.id <= 5);
        let now = 1_795_714_800_000 + 20 * DAY_MS;
        let usage = from_entries(&entries, &[], now);
        assert_eq!(usage.dictations, 5);
        assert_eq!(usage.last_30_days, 3);
        assert_eq!(usage.apps[0], ("Slack".to_owned(), 2));
        assert_eq!(usage.apps.len(), 4);
        assert_eq!(usage.code_share, 0);
        assert_eq!(usage.long_share, 0);
        assert!(usage.median_words > 5 && usage.median_words < 20);
        // Said once, never fixed: not yet a term.
        assert!(usage.terms.is_empty());
        assert!(usage.fixes.is_empty());
        let known = from_entries(&entries, &["openrouter".into()], now);
        assert!(known.terms.is_empty());
        let allowed = Allowed(());
        let read = read(&allowed, &Sources::at(&root), &[], now);
        assert!(read.meetings >= 4);
        assert!(
            read.meeting_languages
                .iter()
                .any(|(language, _)| *language == Language::Russian)
        );
    }

    /// Terms said often in Russian, the words cleanup keeps fixing and how
    /// they were heard, the code editors' share, the long ones.
    #[test]
    fn terms_fixes_and_shares_come_from_the_words() {
        let long = "слово ".repeat(LONG_WORDS);
        let entries = [
            entry(
                1,
                "Ghostty",
                "открой гитхаб и сделай пул реквест",
                "открой GitHub и сделай pull request",
            ),
            entry(2, "Cursor", "запушь на гитхаб", "запушь на GitHub"),
            entry(
                3,
                "Slack",
                "поставь Docker и Docker Compose",
                "поставь Docker и Docker Compose",
            ),
            entry(4, "Telegram", &long, &long),
            entry(5, "Notes", "the API is fine", "the API is fine"),
        ];
        let known = ["docker".to_owned()];
        let usage = from_entries(&entries, &known, 10_000);
        assert_eq!(usage.code_share, 40);
        assert_eq!(usage.long_share, 20);
        assert_eq!(
            usage.fixes,
            [Fix {
                heard: "гитхаб".into(),
                written: "GitHub".into(),
                count: 2,
            }]
        );
        let github = usage
            .terms
            .iter()
            .find(|term| term.word == "GitHub")
            .unwrap();
        assert_eq!(
            (github.count, github.heard.as_slice()),
            (2, ["гитхаб".to_owned()].as_slice())
        );
        // Said once but fixed: a term, with how it was heard, its words
        // as one.
        let pull = usage
            .terms
            .iter()
            .find(|term| term.word == "pull request")
            .unwrap();
        assert_eq!(pull.heard, ["пул реквест"]);
        assert!(usage.terms.iter().all(|term| term.word != "pull"));
        assert!(usage.terms.iter().all(|term| term.word != "Docker"));
        // `API` once in English: not yet.
        assert!(usage.terms.iter().all(|term| term.word != "API"));
        let lines = lines(&usage).join("\n");
        assert!(lines.contains("GitHub x2 (heard as \"гитхаб\")"), "{lines}");
        assert!(!lines.contains("открой"), "{lines}");
    }
}
