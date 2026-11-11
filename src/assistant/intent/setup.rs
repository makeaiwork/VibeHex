//! Whether a message asks for VibeHex to be set up for the person as a
//! whole ("Помоги мне настроить VibeHex под меня", "set VibeHex up for
//! me"), which guided setup answers with three questions and one list of
//! changes, rather than how one setting works. Read from the words alone.
//!
//! A message is a setup request when it says what to turn on without
//! naming a thing ("what should I turn on for…"), or when a setting-up
//! verb is all it holds besides the person, VibeHex, and small words:
//! "помоги настроить" is one, "как настроить горячую клавишу" is a guide
//! question about the shortcut.

use super::{has_phrase, starts, words};

/// Setting-up verbs; `=` ends a word that must match whole.
const VERBS: &[&str] = &[
    "настро",
    "настраива",
    "подстро",
    "сконфигур",
    "конфигур",
    "configur",
    "set=",
    "setup=",
    "personali",
    "customi",
    "tune=",
    "tailor",
    "adjust=",
];

/// Words that leave a request about VibeHex as a whole.
const SMALL: &[&str] = &[
    // The person and the app.
    "me",
    "my",
    "myself",
    "i",
    "you",
    "for",
    "up",
    "it",
    "the",
    "a",
    "app",
    "vibehex",
    "hex",
    "s",
    "everything",
    "all",
    "please",
    "help",
    "can",
    "could",
    "would",
    "how",
    "do",
    "to",
    "want",
    "like",
    "needs",
    "workflow",
    "way",
    "work",
    "let",
    "us",
    "мне",
    "меня",
    "мой",
    "моих",
    "для",
    "под",
    "себя",
    "себе",
    "помоги",
    "помогите",
    "помочь",
    "приложение",
    "приложения",
    "программу",
    "его",
    "это",
    "всё",
    "все",
    "пожалуйста",
    "можешь",
    "можете",
    "давай",
    "давайте",
    "как",
    "хочу",
    "нужно",
    "надо",
    "вайбхекс",
    "мои",
    "нужды",
    "работы",
    "работу",
    "бы",
];

/// Asking what to turn on, whatever it is for.
const WHAT_TO_TURN_ON: &[&str] = &[
    "what= should= i= turn= on=",
    "what= should= i= enable",
    "what= should= i= switch= on=",
    "what= do= i= turn= on=",
    "what= to= turn= on=",
    "what= settings= should= i=",
    "что= мне= включить=",
    "что= включить=",
    "что= стоит= включить=",
    "что= мне= стоит= включить=",
    "что= нужно= включить=",
    "что= мне= нужно= включить=",
    "что= мне= настроить=",
];

/// Asking which ones, of what to turn on or set up for them: «какие Labs
/// мне стоит включить», "which Labs features should I turn on", «что
/// посоветуешь настроить». Each list must have a word in the message.
const WHICH: &[&str] = &[
    "какие=",
    "какую=",
    "какой=",
    "каких=",
    "что=",
    "which=",
    "what=",
];
const TURN_ON: &[&str] = &[
    "включить=",
    "включать=",
    "настроить=",
    "настраивать=",
    "поменять=",
    "turn= on=",
    "turning= on=",
    "switch= on=",
    "enable",
    "set= up=",
];
const ADVICE: &[&str] = &[
    "стоит=",
    "мне=",
    "посовет",
    "совету",
    "порекоменд",
    "рекоменд",
    "should=",
    "worth=",
    "recommend",
    "for= me=",
];

/// Whether `message` asks for VibeHex to be set up for the person.
pub fn setup(message: &str) -> bool {
    let words = words(message);
    if WHAT_TO_TURN_ON
        .iter()
        .any(|phrase| has_phrase(&words, phrase))
    {
        return true;
    }
    let has = |list: &[&str]| list.iter().any(|phrase| has_phrase(&words, phrase));
    if has(WHICH) && has(TURN_ON) && has(ADVICE) {
        return true;
    }
    let verb = |word: &String| VERBS.iter().any(|verb| starts(word, verb));
    words.iter().any(verb)
        && words
            .iter()
            .all(|word| verb(word) || SMALL.contains(&word.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asking_to_be_set_up_is_a_setup_request() {
        for message in [
            "Помоги мне настроить VibeHex под меня",
            "помоги настроить",
            "Настрой под меня",
            "Настрой VibeHex под меня, пожалуйста",
            "Хочу настроить приложение под себя",
            "Set VibeHex up for me",
            "set it up for me",
            "Help me configure",
            "Help me configure VibeHex",
            "Can you set everything up for me?",
            "Personalize VibeHex for my workflow",
            "What should I turn on for meetings in Russian?",
            "what should I enable?",
            "Что мне включить для созвонов?",
            "Что стоит включить?",
            "Which Labs features are worth turning on for me?",
            "Which Labs features should I turn on?",
            "Какие Labs мне стоит включить?",
            // The start's example, as the window asks it in Russian.
            "Какие функции Лаборатории мне стоит включить?",
            "Что мне включить?",
            "Что посоветуешь настроить?",
            "What do you recommend I set up?",
        ] {
            assert!(setup(message), "{message}");
        }
    }

    /// How one thing works stays a guide question.
    #[test]
    fn a_question_about_one_setting_is_not() {
        for message in [
            "Как настроить горячую клавишу диктовки?",
            "Set up for Russian programming",
            "How do I set up meetings?",
            "Help me configure the shortcut",
            "What does Labs › Live text turn on?",
            "Что включает Russian vibe coding?",
            "Что мне делать, если не включается микрофон?",
            "Which speech model is best for Russian?",
            "Как включить запись встреч?",
            "Где настройки?",
            "Settings",
            "Помоги",
            "Can Zoom calls record by themselves, without asking?",
            "Как сделать, чтобы клод код писался по-английски",
        ] {
            assert!(!setup(message), "{message}");
        }
    }
}
