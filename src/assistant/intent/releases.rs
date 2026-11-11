//! Whether a question asks what changed in VibeHex ("Что нового?", "What
//! changed in the last update?", "что появилось в версии 3.3.0-vibe.5",
//! "changelog", "when was Files added?"), which VibeHex's release notes
//! answer (`assistant::releases`), never a search of the person's
//! recordings. Read from the words alone.
//!
//! A question is about releases when it asks what's new, names the
//! changelog or release notes or a version, or names a release, update, or
//! version together with something new, changed, or added. One that also
//! names meetings, calls, dictations, or files without a version or the app
//! stays with them ("what changed in the release meeting?").

use super::{DICTATIONS, FILES, MEETINGS, SAID, has_phrase, starts, words};
use crate::assistant::releases::Asked;

/// Asking what's new, whatever it names; `=` ends a word that must match whole.
const WHATS_NEW: &[&str] = &[
    "what= s= new=",
    "whats= new=",
    "what= is= new=",
    "what= new=",
    "anything= new=",
    "что= нового=",
    "что= новое=",
    "что= новенького=",
    "что= там= нового=",
    "чего= нового=",
    "что= есть= нового=",
    "нововведени",
];

/// The changelog by name.
const CHANGELOG: &[&str] = &[
    "changelog",
    "change= log=",
    "release= notes=",
    "patch= notes=",
    "release= history=",
    "version= history=",
    "чейнджлог",
    "ченджлог",
    "список изменени",
    "истори изменени",
    "истори верси",
    "истори релиз",
];

/// A release, an update, or a version.
const RELEASE: &[&str] = &[
    "release",
    "update=",
    "updates=",
    "updated=",
    "upgrade",
    "version",
    "build=",
    "релиз",
    "обновлени",
    "обновил",
    "апдейт",
    "верси",
    "сборк",
];

/// Something new, changed, added, or fixed.
const CHANGED: &[&str] = &[
    "new=",
    "changed=",
    "changes=",
    "change=",
    "added=",
    "different",
    "fixed=",
    "came=",
    "missed=",
    "нов",
    "изменил",
    "изменени",
    "поменял",
    "появил",
    "добавил",
    "добавлен",
    "исправ",
    "пропустил",
];

/// Asking when something came: the whole history.
const WHEN: &[&str] = &[
    "when= was=",
    "when= did=",
    "since= when=",
    "which= version=",
    "what= version=",
    "which= release=",
    "когда= появил",
    "когда= добавил",
    "когда= добавили=",
    "в= какой= версии=",
    "с= какой= версии=",
    "в= каком= релизе=",
    "в= каком= обновлении=",
];

/// The app itself, which keeps a question with its releases.
const APP: &[&str] = &[
    "vibehex",
    "hex=",
    "app=",
    "вайбхекс",
    "приложени",
    "программ",
];

/// What a question about VibeHex's releases asks for, if it is one.
pub fn releases(question: &str) -> Option<Asked> {
    let words = words(question);
    let any = |stems: &[&str]| {
        words
            .iter()
            .any(|word| stems.iter().any(|stem| starts(word, stem)))
    };
    let phrase = |phrases: &[&str]| phrases.iter().any(|phrase| has_phrase(&words, phrase));
    let version = version(question);
    let release = any(RELEASE);
    let changed = any(CHANGED);
    let changelog = phrase(CHANGELOG);
    let when = phrase(WHEN) && changed;
    // "When was Files added?" names the Files feature, not a file.
    let recordings = any(MEETINGS) || any(DICTATIONS) || (any(FILES) && !when);
    if recordings && version.is_none() && !any(APP) {
        return None;
    }
    // "What did we decide about the release?" is about what was said.
    if any(SAID) && !changed && version.is_none() {
        return None;
    }
    let asks = phrase(WHATS_NEW) || changelog || when || (release && changed) || version.is_some();
    asks.then_some(Asked {
        version,
        history: changelog || when,
    })
}

/// The version a question names: `3.3.0-vibe.5`, `v3.3.0-vibe.5`,
/// `vibe.5`, `vibe 5`; a plain `3.3.0` too.
fn version(question: &str) -> Option<String> {
    let lower = question.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || ",;:!?()«»\"'“”".contains(c))
        .map(|token| token.trim_end_matches('.'))
        .filter(|token| !token.is_empty())
        .collect();
    for (index, token) in tokens.iter().enumerate() {
        let token = token
            .strip_prefix('v')
            .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap_or(token);
        let (stable, build) = token.split_once("-vibe.").unwrap_or((token, ""));
        let parts: Vec<&str> = stable.split('.').collect();
        let numeric = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if parts.len() == 3 && parts.iter().all(|part| numeric(part)) {
            return Some(if numeric(build) {
                format!("{stable}-vibe.{build}")
            } else {
                stable.to_owned()
            });
        }
        // "vibe.5", "vibe5", "vibe 5", "вайб 5".
        for name in ["vibe", "вайб"] {
            let Some(rest) = token.strip_prefix(name) else {
                continue;
            };
            let rest = rest.trim_start_matches(['.', '-']);
            let number = if rest.is_empty() {
                tokens.get(index + 1).copied().unwrap_or("")
            } else {
                rest
            };
            if numeric(number) {
                return Some(format!("vibe.{number}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(question: &str) -> Asked {
        releases(question).unwrap_or_else(|| panic!("{question} isn't about releases"))
    }

    #[test]
    fn asking_whats_new_is_about_releases() {
        for question in [
            "Что нового?",
            "Что нового в последнем релизе?",
            "Что изменилось в последнем обновлении?",
            "Что изменилось в последнем релизе?",
            "Что нового в VibeHex?",
            "Я обновился, что поменялось?",
            "What's new?",
            "Whats new in VibeHex?",
            "What changed in the latest update?",
            "What's new since my version?",
            "Show me the changelog",
            "Release notes",
            "Какие нововведения в последней версии?",
            "Что добавили в новой версии?",
            // What "Updated to <version>" asks, in English and Russian.
            "What's new since my last version?",
            "Что нового с моей прошлой версии?",
        ] {
            let asked = asked(question);
            assert_eq!(asked.version, None, "{question}");
        }
    }

    #[test]
    fn a_named_version_is_read() {
        assert_eq!(
            asked("Что появилось в версии 3.3.0-vibe.5?")
                .version
                .as_deref(),
            Some("3.3.0-vibe.5")
        );
        assert_eq!(
            asked("What changed in v3.3.0-vibe.6").version.as_deref(),
            Some("3.3.0-vibe.6")
        );
        assert_eq!(asked("Что в vibe.4?").version.as_deref(), Some("vibe.4"));
        assert_eq!(
            asked("Что нового в вайб 6").version.as_deref(),
            Some("vibe.6")
        );
    }

    #[test]
    fn when_something_came_reads_the_history() {
        for question in [
            "When was Files added?",
            "Which version added Stop shortcut?",
            "Когда появилась функция Files?",
            "В какой версии добавили Stop shortcut?",
            "Покажи историю изменений",
            "changelog",
        ] {
            assert!(asked(question).history, "{question}");
        }
        assert!(!asked("Что нового?").history);
    }

    /// Updating, the version they run, and their own recordings stay
    /// where they were.
    #[test]
    fn other_questions_are_not_about_releases() {
        for question in [
            "Как обновить VibeHex?",
            "How do I update?",
            "Which version do I have?",
            "Почему VibeHex не обновляется?",
            "Can I turn off updates?",
            "What did we decide about the release?",
            "What changed in the release planning meeting?",
            "Что нового обсуждали на встрече вчера?",
            "Что я диктовал про новый релиз?",
            "Можно сказать «новая строка», чтобы был перенос строки?",
            "Summarize my calls with Anna last week",
            "What's in my budget draft file?",
            "Which speech model is best for Russian?",
            "Помоги мне настроить VibeHex под меня",
        ] {
            assert_eq!(releases(question), None, "{question}");
        }
    }

    /// No guide question of the live eval reads as one about releases.
    #[test]
    fn the_guide_questions_stay_with_the_guide() {
        let text = include_str!("../../../tests/fixtures/assistant/questions.jsonl");
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let line: serde_json::Value = serde_json::from_str(line).unwrap();
            let question = line["q"].as_str().unwrap();
            assert_eq!(releases(question), None, "{question}");
        }
    }

    /// A releases question never searches the recordings.
    #[test]
    fn a_releases_question_is_no_recordings_question() {
        let now = super::super::day::Day::new(2026, 11, 10)
            .midnight(0)
            .unwrap();
        for question in [
            "Что нового в последнем релизе?",
            "Что изменилось в последнем обновлении?",
            "What changed in the last release?",
            "Что я пропустил в последних обновлениях?",
        ] {
            assert_eq!(super::super::read(question, now), None, "{question}");
        }
    }
}
