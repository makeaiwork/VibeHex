//! Whether a message asks to be set up like the person who makes VibeHex
//! («сделай настройки как у Дениса», «как у DenisKey», "set me up like
//! Denis", "like the developer"), which the app answers with the owner's
//! preset (`assistant::setup::preset`). Read from the words alone.

use super::{starts, words};

/// Words that say "the way someone has it".
const LIKE: &[&str] = &["как=", "like=", "as="];

/// Words that may sit between "like" and the name.
const BETWEEN: &[&str] = &["у=", "the=", "its=", "his=", "vibehex", "hex="];

/// The owner by name, or as the one who makes VibeHex.
const NAMES: &[&str] = &[
    "денис",
    "denis",
    "разработчик",
    "автор",
    "создател",
    "developer",
    "author",
    "creator",
    "maker",
];

/// Whether `message` asks for the owner's settings.
pub fn preset(message: &str) -> bool {
    let words = words(message);
    let named = |word: &String| NAMES.iter().any(|name| starts(word, name));
    words.iter().enumerate().any(|(at, word)| {
        LIKE.iter().any(|like| starts(word, like))
            && words[at + 1..]
                .iter()
                .find(|word| !BETWEEN.iter().any(|between| starts(word, between)))
                .is_some_and(named)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asking_for_the_owners_settings_is_a_preset_request() {
        for message in [
            "сделай настройки как у Дениса",
            "Настрой как у Дениса",
            "как у DenisKey",
            "Сделай как у Денис Кей",
            "хочу как у разработчика",
            "настрой всё как у автора",
            "Set me up like Denis",
            "set me up like the developer",
            "Set VibeHex up like DenisKey has it",
            "configure it like the author",
        ] {
            assert!(preset(message), "{message}");
        }
    }

    #[test]
    fn other_questions_are_not() {
        for message in [
            "Помоги мне настроить VibeHex под меня",
            "Как у вас дела?",
            "Set VibeHex up for me",
            "How do I set up meetings like Zoom does?",
            "Кто разработчик VibeHex?",
            "Denis, how does vocabulary work?",
            "Как сделать, чтобы клод код писался по-английски",
        ] {
            assert!(!preset(message), "{message}");
        }
    }
}
