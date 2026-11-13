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
/// The most fixes kept.
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
    /// Terms said in Russian speech, not in Vocabulary (`terms`).
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
    /// How the speech model heard it before cleanup fixed it into
    /// exactly it, most often first.
    pub heard: Vec<String>,
    /// The most times one heard form was fixed into it.
    pub fixed: usize,
    /// With no heard form, how History's raw text has it, as the
    /// vocabulary guard hears it (`sounds`), most often first.
    pub sounds: Vec<String>,
}

impl Term {
    /// Cleanup turned the same heard word into it often enough that
    /// setup offers it checked.
    pub fn sure(&self) -> bool {
        self.fixed >= super::terms::SURE
    }
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
    let mut seen = super::terms::Seen::new();
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
        super::terms::note(&mut seen, &written);
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
    let mut terms = super::terms::pick(&seen, &fixes, known);
    super::sounds::fill(&mut terms, entries, known);
    // Putting ё back (все → всё, еще → ещё) is spelling, the most
    // frequent change there is, and says nothing of how the person works:
    // it would fill the list ahead of кодекс → Codex.
    let yo = |text: &str| text.to_lowercase().replace('ё', "е");
    fixes.retain(|fix| fix.count >= 2 && yo(&fix.heard) != yo(&fix.written));
    // A listed "sounds like" spelling written as its term is Vocabulary at
    // work, in code before cleanup (`dictation::vocabulary_guard`): not a
    // thing cleanup keeps fixing, and nothing the list lacks.
    let listed = super::terms::knowns(known);
    let has = |words: &str| listed.contains(&words.to_lowercase());
    fixes.retain(|fix| !(has(&fix.heard) && has(&fix.written)));
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
    if term.heard.is_empty() && term.sounds.is_empty() {
        format!("{} x{}", term.word, term.count)
    } else if term.heard.is_empty() {
        format!(
            "{} x{} (in History before cleanup as \"{}\")",
            term.word,
            term.count,
            term.sounds.join("\", \"")
        )
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
    (part * 100 + whole / 2).checked_div(whole).unwrap_or(0) as u32
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
        let mut entries = vec![
            entry(
                1,
                "Ghostty",
                "открой гитхаб и сделай пул реквест",
                "открой GitHub и сделай pull request",
            ),
            entry(
                2,
                "Cursor",
                "запушь на гитхаб и пул реквест",
                "запушь на GitHub и pull request",
            ),
            entry(
                3,
                "Slack",
                "поставь Docker и Docker Compose",
                "поставь Docker и Docker Compose",
            ),
            entry(4, "Telegram", &long, &long),
            entry(5, "Notes", "the API is fine", "the API is fine"),
        ];
        entries.push(entry(6, "Notes", "API is fine", "the API is fine"));
        let known = ["docker".to_owned()];
        let usage = from_entries(&entries, &known, 10_000);
        assert_eq!(usage.code_share, 33);
        assert_eq!(usage.long_share, 17);
        assert_eq!(
            usage.fixes[0],
            Fix {
                heard: "гитхаб".into(),
                written: "GitHub".into(),
                count: 2,
            }
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
        assert!(!github.sure(), "fixed twice, not three times");
        // Fixed twice: a term, with how it was heard, its words as one.
        let pull = usage
            .terms
            .iter()
            .find(|term| term.word == "pull request")
            .unwrap();
        assert_eq!(pull.heard, ["пул реквест"]);
        assert!(usage.terms.iter().all(|term| term.word != "pull"));
        assert!(usage.terms.iter().all(|term| term.word != "Docker"));
        // `API` in English sentences only: never.
        assert!(usage.terms.iter().all(|term| term.word != "API"));
        // A spelling Vocabulary lists, written as its term, is the list
        // at work (in code, before cleanup): not a fix cleanup keeps
        // making, and no term the list lacks. What it doesn't list stays.
        let listed = ["GitHub".to_owned(), "гитхаб".to_owned()];
        let with_list = from_entries(&entries, &listed, 10_000);
        let fixed: Vec<&str> = with_list
            .fixes
            .iter()
            .map(|fix| fix.written.as_str())
            .collect();
        assert!(!fixed.contains(&"GitHub"), "{fixed:?}");
        assert!(fixed.contains(&"pull request"), "{fixed:?}");
        assert!(with_list.terms.iter().all(|term| term.word != "GitHub"));
        let lines = lines(&usage).join("\n");
        assert!(lines.contains("GitHub x2 (heard as \"гитхаб\")"), "{lines}");
        assert!(!lines.contains("открой"), "{lines}");
    }

    fn said(id: u64, raw: &str, written: &str) -> HistoryEntry {
        entry(id, "Ghostty", raw, written)
    }

    /// Russian speech with English in it, as a vibe coder dictates: the
    /// ordinary English words in it, a word of a term Vocabulary has, and
    /// the words of a longer fix are no terms, and no heard form comes
    /// from a fix into more than the term. What is one stays: Codex from
    /// кодекс (sure, three times), a name like GitHub, a phrase like pull
    /// request.
    #[test]
    fn only_real_terms_come_from_mixed_speech() {
        let mut entries = Vec::new();
        let mut id = 0;
        let mut add = |raw: &str, written: &str, times: usize| {
            for _ in 0..times {
                id += 1;
                entries.push(said(id, raw, written));
            }
        };
        add(
            "запусти клод код и скажи то есть",
            "запусти Claude Code и скажи to есть",
            3,
        );
        add(
            "открой Claude код в терминале",
            "открой Claude Code в терминале",
            2,
        );
        add(
            "сделай драгн дропом файл and потом use",
            "сделай drag and drop файл and потом use",
            2,
        );
        add(
            "напиши струкче саммари экшен итемс call",
            "напиши Structure summary action items call",
            2,
        );
        add("включи History и Play там", "включи History и Play там", 3);
        add("спроси кодекс про это", "спроси Codex про это", 3);
        add("открой гитхаб сейчас", "открой GitHub сейчас", 1);
        add("запушь в GitHub ветку", "запушь в GitHub ветку", 1);
        add("сделай пул реквест сейчас", "сделай pull request сейчас", 2);
        let known = ["Claude Code".to_owned(), "клод код".to_owned()];
        let usage = from_entries(&entries, &known, 10_000);
        let words: Vec<&str> = usage.terms.iter().map(|term| term.word.as_str()).collect();
        for gone in [
            "to",
            "and",
            "use",
            "Code",
            "Claude",
            "Structure",
            "summary",
            "action",
            "items",
            "call",
            "History",
            "Play",
            "drag",
            "drop",
        ] {
            assert!(!words.contains(&gone), "{gone} in {words:?}");
        }
        let codex = &usage.terms[0];
        assert_eq!(codex.word, "Codex", "{words:?}");
        assert_eq!(codex.heard, ["кодекс"]);
        assert!(codex.sure());
        let github = usage
            .terms
            .iter()
            .find(|term| term.word == "GitHub")
            .unwrap();
        assert_eq!(github.heard, ["гитхаб"]);
        assert!(!github.sure());
        let pull = usage
            .terms
            .iter()
            .find(|term| term.word == "pull request")
            .unwrap();
        assert_eq!(pull.heard, ["пул реквест"]);
        // A phrase of ordinary words cleanup keeps writing stays one, as
        // a whole; never its words.
        let drag = usage
            .terms
            .iter()
            .find(|term| term.word == "drag and drop")
            .unwrap();
        assert_eq!(drag.heard, ["драгн дропом"]);
        for term in &usage.terms {
            for heard in &term.heard {
                assert!(
                    !heard.eq_ignore_ascii_case(&term.word),
                    "{} heard as itself",
                    term.word
                );
            }
        }
    }

    /// The owner's real History against their Vocabulary, read-only:
    /// `HEX_USAGE_HISTORY=… HEX_USAGE_SETTINGS=… cargo test --bin hex
    /// owners_terms -- --ignored --nocapture` prints the terms.
    #[test]
    #[ignore = "reads a real History named by HEX_USAGE_HISTORY"]
    fn owners_terms() {
        let history = std::env::var("HEX_USAGE_HISTORY").unwrap();
        let entries = crate::history::read_snapshot(std::path::Path::new(&history)).unwrap();
        let known: Vec<String> = std::env::var("HEX_USAGE_SETTINGS")
            .ok()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|settings| settings.get("vocabulary").cloned())
            .and_then(|vocabulary| vocabulary.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .flat_map(|entry| {
                let term = entry.get("term").and_then(serde_json::Value::as_str);
                let sounds = entry
                    .get("sounds_like")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                term.map(str::to_owned).into_iter().chain(
                    sounds
                        .into_iter()
                        .filter_map(|sound| sound.as_str().map(str::to_owned)),
                )
            })
            .collect();
        let usage = from_entries(&entries, &known, u64::MAX / 2);
        for term in &usage.terms {
            println!(
                "{} x{} fixed {}{} heard {:?} in History {:?}",
                term.word,
                term.count,
                term.fixed,
                if term.sure() { " SURE" } else { "" },
                term.heard,
                term.sounds
            );
        }
        for line in lines(&usage) {
            println!("{line}");
        }
        let path = std::env::var("HEX_USAGE_SETTINGS").unwrap();
        let (settings, _) =
            crate::app_settings::AppSettings::read_from(std::path::Path::new(&path)).unwrap();
        let candidates = crate::assistant::setup::since::candidates(&usage);
        let state = crate::assistant::facts::State::default();
        let plan = crate::assistant::setup::plan::with_terms(
            Vec::new(),
            &[],
            &candidates,
            &settings,
            &state,
        );
        for item in plan.iter().flat_map(|card| &card.items) {
            println!("plan: {} {}", item.included, item.to);
        }
    }
}
