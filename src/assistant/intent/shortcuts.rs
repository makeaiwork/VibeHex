//! Whether a message asks for help with the person's shortcuts ("help me
//! set up my shortcuts", «помоги настроить горячие клавиши», "which keys
//! do I have"), which the app answers with the shortcuts card under the
//! model's words (`changes::shortcuts`), and which of them: one named
//! ("how do I change the dictation key?") narrows the card to its row.
//! Read from the words alone.
//!
//! It takes a word for shortcuts and a word that asks to see, set, or
//! change them; a message about why one doesn't work, or about an API
//! key, is a guide question like any other.

use super::{has_phrase, starts, words};

/// Words for shortcuts; `=` ends a word that must match whole.
const NOUNS: &[&str] = &[
    "shortcut",
    "hotkey",
    "hot= key",
    "hot= keys",
    "keybinding",
    "key= binding",
    "keyboard= shortcut",
    "key=",
    "keys=",
    "горяч",
    "сочетани",
    "шорткат",
    "хотке",
    "клавиш",
];

/// Asking to see, set up, or change them.
const ASKS: &[&str] = &[
    "set= up",
    "setup",
    "set=",
    "configur",
    "change",
    "rebind",
    "remap",
    "assign",
    "record",
    "customi",
    "pick",
    "choose",
    "which",
    "what=",
    "what's",
    "whats",
    "show",
    "list",
    "help",
    "my=",
    "настро",
    "помен",
    "измен",
    "смен",
    "назнач",
    "переназнач",
    "постав",
    "задат",
    "выбра",
    "какие",
    "какая",
    "какой",
    "какое",
    "какую",
    "каким",
    "каких",
    "покаж",
    "помоги",
    "помогите",
    "мои=",
    "мою=",
    "мой=",
];

/// Words that make it about something else: a shortcut that fails, or a
/// key that is a secret.
const ELSE: &[&str] = &[
    "work",
    "working",
    "broken",
    "why",
    "api",
    "openrouter",
    "license",
    "token",
    "работа",
    "почему",
    "сломал",
    "ключ",
];

/// The rows a named shortcut narrows the card to.
const NAMED: &[(&[&str], &[&str])] = &[
    (&["paste", "встав"], &["paste_last_hotkey"]),
    (&["cancel", "отмен"], &["cancel_hotkey"]),
    (&["stop=", "останов"], &["stop_hotkey"]),
    (&["annotat", "аннотац", "рису"], &["capture.hotkey"]),
    (
        &["screenshot", "скриншот", "снимк"],
        &["screenshots.hotkey"],
    ),
    (&["vocabular", "словар"], &["vocabulary_add_hotkey"]),
    (&["second=", "втор"], &["second_dictation_hotkey"]),
    (&["mode=", "режим", "double", "двойн"], &["shortcut_mode"]),
    (&["dictat", "диктов", "диктуй"], &["dictation_hotkey"]),
];

/// The shortcuts `message` asks about: `Some` of none for all of them, of
/// the keys it names when it names any, `None` when it doesn't ask.
pub fn shortcuts(message: &str) -> Option<Vec<&'static str>> {
    let words = words(message);
    let has = |list: &[&str]| list.iter().any(|phrase| has_phrase(&words, phrase));
    if !has(NOUNS) || !has(ASKS) || has(ELSE) {
        return None;
    }
    // The first name wins: "the paste last dictation shortcut" is paste.
    let named = words.iter().find_map(|word| {
        NAMED
            .iter()
            .find(|(stems, _)| stems.iter().any(|stem| starts(word, stem)))
            .map(|(_, keys)| keys.to_vec())
    });
    Some(named.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asking_for_help_with_shortcuts_gets_them_all() {
        for message in [
            "Can you help me set up my shortcuts?",
            "help me set up my shortcuts",
            "Which keys do I have?",
            "What are my shortcuts?",
            "Show my hotkeys",
            "I want to change my keyboard shortcuts",
            "Помоги настроить горячие клавиши",
            "помоги мне настроить сочетания клавиш",
            "Какие у меня горячие клавиши?",
            "Хочу поменять хоткеи",
            "покажи мои шорткаты",
        ] {
            assert_eq!(shortcuts(message), Some(Vec::new()), "{message}");
        }
    }

    #[test]
    fn asking_about_one_shortcut_gets_its_row() {
        for (message, key) in [
            ("How do I change the dictation key?", "dictation_hotkey"),
            ("Как поменять клавишу диктовки?", "dictation_hotkey"),
            (
                "Как настроить горячую клавишу диктовки?",
                "dictation_hotkey",
            ),
            (
                "Set a shortcut for paste last dictation",
                "paste_last_hotkey",
            ),
            (
                "Какое сочетание для вставки последней диктовки?",
                "paste_last_hotkey",
            ),
            ("Change the shortcut mode", "shortcut_mode"),
        ] {
            assert_eq!(shortcuts(message), Some(vec![key]), "{message}");
        }
    }

    #[test]
    fn other_questions_are_not() {
        for message in [
            "Set me up like Denis",
            "Set VibeHex up for me",
            "Помоги мне настроить VibeHex под меня",
            "Where do I put my OpenRouter key?",
            "What is an API key?",
            "Why doesn't my shortcut work in Terminal?",
            "Почему не работает горячая клавиша?",
            "How does dictation work?",
            "Какая модель лучше для русского?",
            "What changed in the latest update?",
            "Что нового в последнем релизе?",
            "Shortcuts",
        ] {
            assert_eq!(shortcuts(message), None, "{message}");
        }
    }
}
