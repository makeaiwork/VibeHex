//! A follow-up that only refers back ("ответь на последний вопрос", «а
//! там ещё что-то?», "tell me more", "and the other ones") says nothing of
//! recordings itself, yet asks about the ones the conversation was about.
//! It carries the newest earlier question's intent (kinds, words, time),
//! so the search runs again for it; "answer my last question" re-runs
//! that question's search. A new question, or one asking how VibeHex
//! works, never carries anything.

use super::{HOW, Intent, SETUP, has_phrase, read, starts, terms, words};

/// Naming the earlier question; `=` ends a word that must match whole.
const QUESTION: &[&str] = &[
    "последн вопрос",
    "предыдущ вопрос",
    "прошл вопрос",
    "тот= вопрос",
    "ответ на= вопрос",
    "ответ на= мой= вопрос",
    "мой= вопрос",
    "last= question",
    "previous= question",
    "earlier= question",
    "answer my= question",
    "answer that= question",
    "answer the= question",
    "my= question",
];

/// Asking for more of the same, or again.
const MORE: &[&str] = &[
    "еще=",
    "подробн",
    "детальн",
    "остальн",
    "други",
    "дальше=",
    "продолж",
    "больше=",
    "раз=",
    "again=",
    "more=",
    "else=",
    "other=",
    "others=",
    "rest=",
    "continue=",
    "detail",
    "elaborate",
    "expand=",
    "там=",
    "них=",
    "ними=",
];

/// Pointing at what the last answer found while naming what to look for
/// among it: «а в какой из них было про поиск?», "which of them mention
/// the release?".
const AMONG: &[&str] = &[
    "из= них=",
    "среди= них=",
    "в= них=",
    "of= them=",
    "of= those=",
    "of= these=",
    "among= them=",
    "among= those=",
];

/// Words a follow-up may also hold without naming anything new.
const SMALL: &[&str] = &[
    "ответ",
    "answer",
    "вопрос",
    "question",
    "задал",
    "спрашивал",
    "ones=",
    "one=",
    "пункт",
    "point=",
    "item",
    "перв",
    "втор",
    "трет",
    "first=",
    "second=",
    "third=",
    "нибудь=",
    "что=",
    "try=",
    "попробу",
    "тогда=",
    "go=",
    "on=",
    "said=",
    "мог=",
    "можно=",
    "пожалуйста=",
];

/// Whether `question` only refers back to the conversation.
pub fn refers_back(question: &str) -> bool {
    let words = words(question);
    if QUESTION.iter().any(|phrase| has_phrase(&words, phrase)) {
        return true;
    }
    let how = HOW
        .iter()
        .chain(SETUP)
        .any(|phrase| has_phrase(&words, phrase));
    let more = words
        .iter()
        .any(|word| MORE.iter().any(|stem| starts(word, stem)));
    if how || !more {
        return false;
    }
    // Naming something new makes it a new question.
    terms(&words).iter().all(|term| {
        MORE.iter()
            .chain(SMALL)
            .any(|stem| starts(term, stem.trim_end_matches('=')) || starts(term, stem))
    })
}

/// Whether `question` asks about some of what was found before, naming
/// what: it stays with the same recordings, and looks for its words too.
pub fn narrows(question: &str) -> bool {
    let words = words(question);
    let how = HOW
        .iter()
        .chain(SETUP)
        .any(|phrase| has_phrase(&words, phrase));
    !how && AMONG.iter().any(|phrase| has_phrase(&words, phrase))
}

/// The words `question` names itself, past the ones any follow-up holds.
fn own_terms(question: &str) -> Vec<String> {
    terms(&words(question))
        .into_iter()
        .filter(|term| {
            !MORE
                .iter()
                .chain(SMALL)
                .any(|stem| starts(term, stem.trim_end_matches('=')) || starts(term, stem))
        })
        .collect()
}

/// What `question` reads: its own intent, or, when it only refers back,
/// that of the newest `earlier` question (oldest first) about recordings,
/// past other follow-ups. None for a question about VibeHex, and for a
/// follow-up to one.
pub fn carried(question: &str, earlier: &[&str], now_ms: u64) -> Option<Intent> {
    carry(question, earlier, |asked| read(asked, now_ms))
}

/// What `question` reads by `read`: its own, or, when it only refers back,
/// the newest `earlier` question's, past other follow-ups.
pub(super) fn carry(
    question: &str,
    earlier: &[&str],
    read: impl Fn(&str) -> Option<Intent>,
) -> Option<Intent> {
    if let Some(intent) = read(question) {
        return Some(intent);
    }
    let among = narrows(question);
    if !refers_back(question) && !among {
        return None;
    }
    for asked in earlier.iter().rev() {
        if let Some(mut intent) = read(asked) {
            if among {
                // What it asks about among them is searched for too.
                for term in own_terms(question) {
                    if !intent.terms.contains(&term) {
                        intent.terms.push(term);
                    }
                }
            }
            return Some(intent);
        }
        if !refers_back(asked) && !narrows(asked) {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASKED: &str = "А можешь найти последние мои диктовки, где я говорил о VibeHex?";

    fn now() -> u64 {
        super::super::day::Day::new(2026, 11, 10)
            .midnight(0)
            .unwrap()
    }

    #[test]
    fn a_follow_up_carries_the_earlier_search() {
        let first = read(ASKED, now()).unwrap();
        for follow_up in [
            "А можешь ответить также на последний вопрос, который я задал?",
            "Ответь на мой предыдущий вопрос",
            "ещё",
            "А там ещё что-нибудь было?",
            "Подробнее",
            "Расскажи подробнее про второй пункт",
            "and the other ones?",
            "Answer my last question",
            "Tell me more",
            "Попробуй ещё раз",
        ] {
            assert!(refers_back(follow_up), "{follow_up}");
            assert_eq!(
                carried(follow_up, &[ASKED], now()).as_ref(),
                Some(&first),
                "{follow_up}"
            );
        }
        // Past another follow-up, to the question before it.
        assert_eq!(
            carried("подробнее", &[ASKED, "ещё"], now()),
            Some(first.clone())
        );
    }

    #[test]
    fn a_new_question_or_a_guide_one_carries_nothing() {
        for question in [
            "Tell me more about Labs",
            "Как включить запись встреч?",
            "Which speech model is best for Russian?",
            "Расскажи подробнее, как работает словарь",
            "What else can VibeHex do?",
            "Can I use my ChatGPT plan to clean up dictation?",
        ] {
            assert_eq!(carried(question, &[ASKED], now()), None, "{question}");
        }
        // A follow-up to a guide question stays with the guide.
        for follow_up in ["Answer my last question", "ещё", "подробнее"] {
            assert_eq!(
                carried(
                    follow_up,
                    &[ASKED, "How do I turn on meeting notes?"],
                    now()
                ),
                None,
                "{follow_up}"
            );
            assert_eq!(carried(follow_up, &[], now()), None, "{follow_up}");
        }
    }
}
