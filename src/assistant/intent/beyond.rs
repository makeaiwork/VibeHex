//! Whether a question asked about one recording, in a meeting's or a file's
//! Requests, reaches past it: "как мы решили на прошлой встрече", "сравни с
//! созвоном во вторник", "это было в файле с интервью?", "what did I
//! dictate about this". Such a question also reads the person's other
//! recordings; one about this item alone reads only it, as before.
//!
//! Read from the question's words with the Assistant's own readers of kinds,
//! times, and words to search for, against the item asked about (its kind,
//! when it was, its title and text):
//!
//! - surely beyond: another recording of a kind it names ("прошлой встрече",
//!   "the previous call", "в другом файле", "next meeting"), a kind this item
//!   isn't (a dictation always; a file from a meeting, a meeting from a
//!   file), several of a kind ("на встречах", "my calls"), a time this item
//!   wasn't in when it asks what was said or names a recording ("что мы
//!   решили во вторник", "last week's call"), or a habit ("как обычно",
//!   "всегда", "раньше", "as usual", "last time");
//! - maybe beyond: comparing with nothing else ("сравни с тем, что…"), or a
//!   name the question asks about that this item's text never has, however
//!   speech recognition writes it (`Atlas`, «Kubernetes»): only a search
//!   that finds it elsewhere brings other recordings in, and only once the
//!   person allowed reading them.
//!
//! "This meeting", «в этом файле» keep a question here, and reworking the
//! last answer (translate, shorten, reformat it) never reaches for a name.

use super::{
    DICTATIONS, FEW, FILES, Intent, Kinds, MEETINGS, PERSONAL, has_phrase, latest, latin_forms,
    month, number, starts, terms, weekday, when, words,
};
use crate::assistant::recordings::Kind;

/// The recording a question is asked about.
#[derive(Clone, Copy, Debug)]
pub struct Item<'a> {
    pub kind: Kind,
    /// When it was recorded, and when it ended (a file: when it was added).
    pub start_ms: u64,
    pub end_ms: u64,
    pub title: &'a str,
    /// Its text as the model reads it, for the names it holds.
    pub text: &'a str,
}

/// What `question`, asked about `item`, reads besides it: none when it is
/// about the item alone.
pub fn read(question: &str, item: &Item<'_>, now_ms: u64) -> Option<Intent> {
    let words = words(question);
    let mut kinds = Kinds::default();
    let mut since = None;
    let mut until = None;
    let mut last = None;
    let mut sure = false;
    // Naming this item ("this meeting", «в этом файле») keeps it here.
    let mut this = false;
    for (index, word) in words.iter().enumerate() {
        let Some(named) = kind_of(word) else {
            continue;
        };
        if words[index.saturating_sub(3)..index]
            .iter()
            .any(|word| THIS.contains(&word.as_str()))
        {
            this |= includes(named, item.kind);
            continue;
        }
        match qualifier(&words[..index]) {
            Some(Side::Before) => {
                add(&mut kinds, named);
                until = Some(item.start_ms);
                last = Some(if plural(word) { FEW } else { 1 });
                sure = true;
            }
            Some(Side::After) => {
                add(&mut kinds, named);
                since = Some(item.end_ms.max(item.start_ms + 1));
                sure = true;
            }
            Some(Side::Other) => {
                add(&mut kinds, named);
                sure = true;
            }
            None if !includes(named, item.kind) || plural(word) => {
                add(&mut kinds, named);
                sure = true;
            }
            None => {}
        }
    }
    let compare = COMPARE.iter().any(|phrase| has_phrase(&words, phrase));
    if !this && let Some(habit) = HABITS.iter().find(|(phrase, _)| has_phrase(&words, phrase)) {
        sure = true;
        if habit.1 {
            until = until.or(Some(item.start_ms));
        }
    }
    // A time counts when it is asked what was said then, or a recording
    // is named; a time this item was in only beside another cue.
    if let Some((from, to)) = when(&words, now_ms) {
        let asks = kinds != Kinds::default()
            || words.iter().any(|word| kind_of(word).is_some())
            || PAST.iter().any(|phrase| has_phrase(&words, phrase));
        let inside = item.start_ms < to && item.end_ms.max(item.start_ms) >= from;
        if asks && (!inside || sure || compare) {
            since = Some(from);
            until = Some(to);
            sure = true;
        }
    }
    let mut terms = terms(&words);
    for word in &words {
        if TOPICAL.iter().any(|stem| starts(word, stem)) && !terms.contains(word) {
            terms.push(word.clone());
        }
    }
    // "What did I dictate about this": what this item is about, by its
    // title, when nothing else narrows it.
    if terms.is_empty()
        && since.is_none()
        && ABOUT_THIS.iter().any(|phrase| has_phrase(&words, phrase))
    {
        terms = super::terms(&super::words(item.title));
    }
    let maybe = !sure
        && !this
        && (compare
            || (!REWORK
                .iter()
                .any(|stem| words.iter().any(|w| starts(w, stem)))
                && missing_name(question, item.text)));
    if !sure && !maybe {
        return None;
    }
    Some(Intent {
        kinds: if kinds == Kinds::default() {
            Kinds::ALL
        } else {
            kinds
        },
        since_ms: since,
        until_ms: until,
        latest: latest(&words).or(last),
        spoken: latin_forms(question, &terms),
        terms,
        sure,
        written: None,
    })
}

/// What `question` reads besides `item`: its own reach, or, when it only
/// refers back ("подробнее", "and the others?"), the newest `earlier`
/// question's (oldest first), past other follow-ups.
pub fn carried(question: &str, earlier: &[&str], item: &Item<'_>, now_ms: u64) -> Option<Intent> {
    super::follow::carry(question, earlier, |asked| read(asked, item, now_ms))
}

/// Which way from this item a word points.
enum Side {
    /// Earlier: «прошлой встрече», "the previous call", "last time".
    Before,
    /// Later: «следующем созвоне», "the next meeting".
    After,
    /// Another, either way: «в другом файле», "other meetings".
    Other,
}

/// The side a kind's noun is on, from the word just before it, or the one
/// before that past a count or "our" ("the last two calls", «других наших
/// встречах»).
fn qualifier(before: &[String]) -> Option<Side> {
    let mut words = before.iter().rev();
    let next = words.next()?;
    side(next).or_else(|| {
        let between = number(next).is_some()
            || matches!(next.as_str(), "few" | "couple" | "of")
            || PERSONAL.contains(&next.as_str());
        words.next().filter(|_| between).and_then(|word| side(word))
    })
}

fn side(word: &str) -> Option<Side> {
    const BEFORE: &[&str] = &[
        "прошл",
        "предыдущ",
        "последн",
        "previous",
        "last=",
        "earlier=",
        "former",
        "past=",
        "prior=",
    ];
    const AFTER: &[&str] = &[
        "следующ",
        "будущ",
        "next=",
        "later=",
        "upcoming",
        "following=",
    ];
    const OTHER: &[&str] = &[
        "друг",
        "остальн",
        "ины",
        "иной=",
        "other=",
        "others=",
        "another=",
        "rest=",
        "recent=",
        "недавн",
        "все=",
        "всех=",
        "всем=",
        "all=",
        "every=",
    ];
    let any = |stems: &[&str]| stems.iter().any(|stem| starts(word, stem));
    if any(BEFORE) {
        Some(Side::Before)
    } else if any(AFTER) {
        Some(Side::After)
    } else if any(OTHER) {
        Some(Side::Other)
    } else {
        None
    }
}

/// The kinds a word names: a meeting, a file, a dictation, or any
/// recording ("time" in "last time", «раз» in «в прошлый раз»).
fn kind_of(word: &str) -> Option<Kinds> {
    let named = |stems: &[&str]| stems.iter().any(|stem| starts(word, stem));
    let kinds = Kinds {
        meetings: named(MEETINGS),
        files: named(FILES),
        dictations: named(DICTATIONS),
    };
    if kinds != Kinds::default() {
        return Some(kinds);
    }
    matches!(word, "time" | "раз" | "recording" | "recordings").then_some(Kinds::ALL)
}

fn add(kinds: &mut Kinds, named: Kinds) {
    kinds.meetings |= named.meetings;
    kinds.files |= named.files;
    kinds.dictations |= named.dictations;
}

/// Whether `named` takes in an item of `kind`.
fn includes(named: Kinds, kind: Kind) -> bool {
    match kind {
        Kind::Meeting => named.meetings,
        Kind::File => named.files,
        Kind::Dictation => named.dictations,
    }
}

/// A kind's noun in a form that is surely more than one.
fn plural(word: &str) -> bool {
    const RUSSIAN: &[&str] = &[
        "встречах=",
        "встречам=",
        "встречами=",
        "встреч=",
        "созвонах=",
        "созвонов=",
        "созвонам=",
        "созвонами=",
        "созвоны=",
        "звонках=",
        "звонков=",
        "звонки=",
        "файлах=",
        "файлов=",
        "файлам=",
        "файлы=",
        "записях=",
        "записей=",
        "диктовках=",
        "диктовок=",
    ];
    let latin = word.is_ascii() && word.len() > 3 && word.ends_with('s');
    (latin && kind_of(word).is_some()) || RUSSIAN.iter().any(|form| starts(word, form))
}

/// A name the question asks about, as written (capitalised past a
/// sentence's start, or Latin among Cyrillic, or with digits), that `text`
/// has in none of the ways speech recognition writes it.
fn missing_name(question: &str, text: &str) -> bool {
    let cyrillic = question
        .chars()
        .any(|c| matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё'));
    let text = text.to_lowercase().replace('ё', "е");
    let mut start = true;
    for written in question.split_whitespace() {
        let word = written.trim_matches(|c: char| !c.is_alphanumeric());
        let opens = start;
        start = written.ends_with(['.', '?', '!', ':']);
        if word.chars().count() < 3 {
            continue;
        }
        let capital = word.chars().next().is_some_and(char::is_uppercase);
        let latin = cyrillic && word.chars().all(|c| c.is_ascii_alphanumeric());
        let digits =
            word.chars().any(|c| c.is_ascii_digit()) && word.chars().any(char::is_alphabetic);
        if !((capital && !opens) || latin || digits) {
            continue;
        }
        let lower = word.to_lowercase().replace('ё', "е");
        if ordinary(&lower) {
            continue;
        }
        // A sound in Cyrillic is near enough by its start: «кубернет».
        let mut forms = super::stems(&lower);
        forms.extend(super::spoken::forms(word).iter().map(|form| {
            let keep = form.chars().count().saturating_sub(2).max(4);
            form.chars().take(keep).collect::<String>()
        }));
        if !forms.iter().any(|form| text.contains(form.as_str())) {
            return true;
        }
    }
    false
}

/// A word that names no topic: a question word, a kind, a time, the
/// person, a language.
fn ordinary(word: &str) -> bool {
    super::STOP.contains(&word)
        || PERSONAL.contains(&word)
        || kind_of(word).is_some()
        || number(word).is_some()
        || weekday(word).is_some()
        || month(word, "in").is_some()
        || THIS.contains(&word)
        || LANGUAGES.iter().any(|stem| starts(word, stem))
}

/// Words that point at this item.
const THIS: &[&str] = &[
    "this",
    "этой",
    "этом",
    "этот",
    "эту",
    "эта",
    "этого",
    "этому",
    "текущ",
    "current",
    "данной",
    "данном",
];

/// Comparing with something, which may be elsewhere.
const COMPARE: &[&str] = &[
    "сравни",
    "сравнить",
    "сравнени",
    "сопостав",
    "в= отличие=",
    "отличает",
    "compare",
    "comparison",
    "versus",
    "vs=",
    "differ",
];

/// A habit, which only other recordings show; `true`: those before this
/// item.
const HABITS: &[(&str, bool)] = &[
    ("как= обычно=", false),
    ("обычно=", false),
    ("всегда=", false),
    ("каждый= раз=", false),
    ("каждую= встреч", false),
    ("раньше=", true),
    ("ранее=", true),
    ("прежде=", true),
    ("в= прошлом=", true),
    ("до= сих= пор=", false),
    ("as= usual=", false),
    ("usually=", false),
    ("always=", false),
    ("every= time=", false),
    ("previously=", true),
    ("in= the= past=", true),
    ("so= far=", false),
    ("before= today=", true),
];

/// Asking what was said, done, or decided then, which a time narrows.
const PAST: &[&str] = &[
    "решил",
    "обсужд",
    "обсудил",
    "говорил",
    "сказал",
    "договорил",
    "обещал",
    "диктовал",
    "надиктовал",
    "записал",
    "было=",
    "были=",
    "decid",
    "discuss",
    "said=",
    "talked",
    "agreed",
    "promised",
    "dictated",
    "recorded",
    "did= we=",
    "did= i=",
    "was= said=",
    "were= we=",
];

/// "About this": what the item is about.
const ABOUT_THIS: &[&str] = &[
    "про= это=",
    "об= этом=",
    "о= том=",
    "на= эту= тем",
    "по= этой= тем",
    "по= теме=",
    "about= this=",
    "about= it=",
    "on= this=",
    "this= topic=",
    "the= same=",
    "то= же=",
];

/// Kind words that also name the topic ("the file with the interview").
const TOPICAL: &[&str] = &[
    "интервью",
    "interview",
    "podcast",
    "подкаст",
    "lecture",
    "лекци",
];

/// Reworking the last answer, which names no topic to look for.
const REWORK: &[&str] = &[
    "translat",
    "перевед",
    "переведи",
    "перевод",
    "shorten",
    "сократ",
    "короче",
    "rewrite",
    "rephrase",
    "перепиш",
    "перефраз",
    "reformat",
    "format",
    "оформ",
    "table",
    "таблиц",
    "bullet",
    "списк",
];

/// Languages an answer is asked in, which name no topic.
const LANGUAGES: &[&str] = &[
    "english",
    "russian",
    "german",
    "french",
    "spanish",
    "chinese",
    "английск",
    "русск",
    "немецк",
    "французск",
    "испанск",
    "китайск",
];

#[cfg(test)]
mod tests {
    use super::super::day::Day;
    use super::*;

    /// Tuesday 10 November 2026, noon local.
    fn now() -> u64 {
        Day::new(2026, 11, 10).midnight(0).unwrap() + 12 * 3_600_000
    }

    fn day(year: i64, month: u32, day: u32) -> u64 {
        Day::new(year, month, day).midnight(0).unwrap()
    }

    /// A meeting on Thursday 5 November, 10:00 to 11:00, about a budget.
    fn meeting() -> Item<'static> {
        let start = day(2026, 11, 5) + 10 * 3_600_000;
        Item {
            kind: Kind::Meeting,
            start_ms: start,
            end_ms: start + 3_600_000,
            title: "Arcade budget sync",
            text: "Meeting: Arcade budget sync\nYou 00:01 Anna wants the budget cut by 15 percent. \
                   Them 00:31 Олег отвечает за дизайн и за найм. We ship the login on Friday. \
                   Them 00:40 кубернетис кластер переезжает в декабре.",
        }
    }

    /// A file added on Monday 9 November: an interview transcript.
    fn file() -> Item<'static> {
        let start = day(2026, 11, 9) + 9 * 3_600_000;
        Item {
            kind: Kind::File,
            start_ms: start,
            end_ms: start,
            title: "interview-maria.m4a",
            text: "Maria talks about onboarding and the pricing page. Мария хочет тариф для команд.",
        }
    }

    fn reach(question: &str, item: &Item<'_>) -> Option<Intent> {
        read(question, item, now())
    }

    #[test]
    fn questions_past_this_item_reach_the_other_recordings() {
        for (question, item) in [
            ("Как мы решили по бюджету на прошлой встрече?", meeting()),
            ("Сравни с созвоном во вторник", meeting()),
            ("Это было в файле с интервью?", meeting()),
            ("What did I dictate about this?", meeting()),
            ("What did we decide in the previous meeting?", meeting()),
            ("Что я диктовал про бюджет?", meeting()),
            ("Обсуждали ли мы найм на других встречах?", meeting()),
            ("How does this compare to our other calls?", meeting()),
            ("Что Анна обычно говорит про бюджет?", meeting()),
            ("Мы всегда переносим релиз?", meeting()),
            ("Did Anna say the same last time?", meeting()),
            ("Как в прошлый раз решили с логином?", meeting()),
            ("What did we discuss yesterday?", meeting()),
            ("Что мы обсуждали в октябре?", meeting()),
            ("Was this mentioned in my meetings this month?", file()),
            ("Did Maria come up in any meeting?", file()),
            ("Что говорили про тариф на созвонах?", file()),
            ("Есть ли это в других файлах?", file()),
            ("Что изменилось с прошлого созвона?", meeting()),
            ("Раньше Олег говорил иначе?", meeting()),
            ("Что будет на следующей встрече?", meeting()),
        ] {
            let intent = reach(question, &item).unwrap_or_else(|| panic!("{question} stays"));
            assert!(intent.sure, "{question} is only maybe");
        }
    }

    #[test]
    fn questions_about_this_item_stay_with_it() {
        for (question, item) in [
            ("Кто что пообещал?", meeting()),
            ("Summarize the decisions", meeting()),
            ("Переведи последний ответ на английский", meeting()),
            ("Translate the answer into English", meeting()),
            ("Что сказал Олег в начале?", meeting()),
            ("What did we decide about the budget?", meeting()),
            ("Сделай список задач", meeting()),
            ("Draft a follow-up email to Anna", meeting()),
            ("Who spoke the most?", meeting()),
            ("Перескажи последние 10 минут", meeting()),
            ("What was the last thing Anna said?", meeting()),
            ("Ответь подробнее", meeting()),
            ("What did they say about the previous slide?", meeting()),
            ("Что было до перерыва?", meeting()),
            ("In this meeting, who disagreed?", meeting()),
            ("Что решили в этой встрече про Кубернетис?", meeting()),
            ("What's the main idea of this file?", file()),
            ("Что Мария говорит про тариф в этом файле?", file()),
            ("Какой дедлайн в пятницу?", meeting()),
            ("Сократи до трёх пунктов", meeting()),
            ("What did we decide on Thursday?", meeting()),
            ("Кто такой Олег?", meeting()),
            ("Перескажи последние 10 минут встречи", meeting()),
            ("Какие темы были на всей этой встрече?", meeting()),
        ] {
            assert_eq!(reach(question, &item), None, "{question} reaches");
        }
    }

    #[test]
    fn the_previous_meeting_is_the_newest_before_this_one() {
        let item = meeting();
        let intent = reach("Как мы решили по бюджету на прошлой встрече?", &item).unwrap();
        assert!(intent.kinds.meetings && !intent.kinds.files && !intent.kinds.dictations);
        assert_eq!(
            (intent.since_ms, intent.until_ms),
            (None, Some(item.start_ms))
        );
        assert_eq!(intent.latest, Some(1));
        assert_eq!(intent.terms, ["бюджет"]);
        let next = reach("What will we cover in the next meeting?", &item).unwrap();
        assert_eq!(next.since_ms, Some(item.end_ms));
        let before = reach("Что решили на прошлых встречах?", &item).unwrap();
        assert_eq!(before.latest, Some(FEW));
    }

    #[test]
    fn a_weekday_or_a_kind_narrows_what_it_reads() {
        let tuesday = reach("Сравни с созвоном во вторник", &meeting()).unwrap();
        assert_eq!(
            (tuesday.since_ms, tuesday.until_ms),
            (Some(day(2026, 11, 10)), Some(day(2026, 11, 11)))
        );
        assert!(tuesday.kinds.meetings);
        let interview = reach("Это было в файле с интервью?", &meeting()).unwrap();
        assert!(interview.kinds.files && interview.terms.contains(&"интервью".to_owned()));
        let dictated = reach("Что я диктовал про бюджет?", &meeting()).unwrap();
        assert!(dictated.kinds.dictations && !dictated.kinds.meetings);
        assert_eq!(dictated.terms, ["бюджет"]);
        // "About this": what the item is about, by its title.
        let this = reach("What did I dictate about this?", &meeting()).unwrap();
        assert_eq!(this.terms, ["arcade", "budget"]);
    }

    #[test]
    fn a_name_this_item_never_has_may_be_elsewhere() {
        for question in [
            "А что с проектом Atlas?",
            "What did Boris say about it?",
            "Когда переезжает Postgres?",
        ] {
            let intent = reach(question, &meeting()).unwrap_or_else(|| panic!("{question}"));
            assert!(!intent.sure, "{question} is sure");
        }
        // Here in any spelling: «кубернетис» is Kubernetes.
        assert_eq!(reach("Когда переезжает Kubernetes?", &meeting()), None);
        assert_eq!(reach("What did Anna say?", &meeting()), None);
        // Comparing alone may be elsewhere too.
        assert!(
            !reach("Сравни это с тем, что решили про логин", &meeting())
                .unwrap()
                .sure
        );
    }

    #[test]
    fn a_follow_up_reaches_where_the_question_before_it_did() {
        let item = meeting();
        let first = "Как мы решили по бюджету на прошлой встрече?";
        let reached = reach(first, &item).unwrap();
        for follow_up in ["подробнее", "А там ещё что-нибудь было?", "Tell me more"]
        {
            assert_eq!(
                carried(follow_up, &[first], &item, now()).as_ref(),
                Some(&reached),
                "{follow_up}"
            );
        }
        // After a question about this item, a follow-up stays with it.
        assert_eq!(
            carried("подробнее", &["Кто что пообещал?"], &item, now()),
            None
        );
        assert_eq!(carried("Кто что пообещал?", &[first], &item, now()), None);
    }
}
