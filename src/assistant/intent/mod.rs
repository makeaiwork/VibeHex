//! Whether a question is about the person's own recordings (meetings,
//! Files texts, dictations) rather than about VibeHex, read from its words
//! alone: nothing here opens a recording, so it runs before the person has
//! agreed to anything. A guide question ("Can Zoom calls record by
//! themselves?") must never bring up the consent card.
//!
//! A question is surely about recordings when it names them (meetings,
//! calls, dictations, files) together with a time ("in November", "вчера",
//! "the last five meetings") or with what was said in them ("what did we
//! decide", "сведи", "summarize my calls"); asking how VibeHex works
//! ("how do I", "можно ли", "where are … stored") without a time keeps it
//! with the guide. A question that only names them, or only asks what was
//! said, may be: once the person has agreed, the search decides it.
//!
//! Times are read against a `now` passed in, in local time. A question
//! about VibeHex's releases ("Что нового в последнем релизе?") is never one
//! about recordings (`releases`).

pub mod beyond;
mod day;
mod follow;
pub mod preset;
mod releases;
mod setup;
mod spoken;
mod stop;

use day::{Day, span};
pub use follow::carried;
pub use releases::releases;
pub use setup::setup;
pub use spoken::forms as spoken_forms_of;
use stop::STOP;

/// Which recordings a question names; none named means all.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Kinds {
    pub meetings: bool,
    pub files: bool,
    pub dictations: bool,
}

impl Kinds {
    const ALL: Self = Self {
        meetings: true,
        files: true,
        dictations: true,
    };
}

/// What a question about recordings asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Intent {
    pub kinds: Kinds,
    /// The time it names, `[since, until)` in milliseconds.
    pub since_ms: Option<u64>,
    pub until_ms: Option<u64>,
    /// "The last five meetings": the newest this many.
    pub latest: Option<usize>,
    /// The words to search for, without times, kinds, and question words;
    /// Russian words cut to a stem the search's prefix match extends.
    pub terms: Vec<String>,
    /// For a Latin term, the ways speech recognition may write it
    /// (`vibehex`: «вайб хекс», "vibe hex"), which the search counts too.
    pub spoken: Vec<(String, Vec<String>)>,
    /// Surely about recordings; otherwise only if a search finds them.
    pub sure: bool,
}

/// The question's intent when it may be about recordings.
pub fn read(question: &str, now_ms: u64) -> Option<Intent> {
    if releases(question).is_some() {
        return None;
    }
    let words = words(question);
    let mut kinds = Kinds::default();
    let mut named = false;
    for word in &words {
        if MEETINGS.iter().any(|stem| starts(word, stem)) {
            kinds.meetings = true;
        } else if DICTATIONS.iter().any(|stem| starts(word, stem)) {
            kinds.dictations = true;
        } else if FILES.iter().any(|stem| starts(word, stem)) {
            kinds.files = true;
        } else {
            continue;
        }
        named = true;
    }
    let said = words
        .iter()
        .any(|word| SAID.iter().any(|stem| starts(word, stem)));
    let personal = words.iter().any(|word| PERSONAL.contains(&word.as_str()));
    let how = HOW.iter().any(|phrase| has_phrase(&words, phrase));
    // Saying how dictation should work for them ("мне важно удобство
    // диктовки", "accuracy matters") speaks of dictating, not of what was
    // dictated, unless they also ask to find or recall it.
    let recall = RECALL.iter().any(|phrase| has_phrase(&words, phrase));
    let setup = !recall && SETUP.iter().any(|phrase| has_phrase(&words, phrase));
    let guide = how || setup;
    let when = when(&words, now_ms);
    let latest = latest(&words);
    let timed = when.is_some() || latest.is_some();
    let sure = named && (timed || said) && !(guide && !timed);
    let maybe = (named && personal && !guide) || (said && personal && !guide) || (timed && said);
    if !sure && !maybe {
        return None;
    }
    let (since_ms, until_ms) = when.unzip();
    let terms = terms(&words);
    Some(Intent {
        kinds: if named { kinds } else { Kinds::ALL },
        since_ms,
        until_ms,
        latest,
        spoken: latin_forms(question, &terms),
        terms,
        sure,
    })
}

/// Each Latin term's spoken forms, read from the word as written, so
/// `VibeHex` comes apart at its capitals.
fn latin_forms(question: &str, terms: &[String]) -> Vec<(String, Vec<String>)> {
    let mut spoken: Vec<(String, Vec<String>)> = Vec::new();
    for written in question.split(|c: char| !c.is_alphanumeric()) {
        let term = written.to_lowercase();
        if !terms.contains(&term) || spoken.iter().any(|(known, _)| *known == term) {
            continue;
        }
        let forms = spoken::forms(written);
        if !forms.is_empty() {
            spoken.push((term, forms));
        }
    }
    spoken
}

/// Lowercase words, `ё` read as `е`.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase().replace('ё', "е"))
        .collect()
}

fn starts(word: &str, stem: &str) -> bool {
    match stem.strip_suffix('=') {
        Some(exact) => word == exact,
        None => word.starts_with(stem),
    }
}

fn has_phrase(words: &[String], phrase: &str) -> bool {
    let phrase: Vec<&str> = phrase.split(' ').collect();
    words.windows(phrase.len()).any(|window| {
        window
            .iter()
            .zip(&phrase)
            .all(|(word, stem)| starts(word, stem))
    })
}

/// Word starts naming meetings; `=` ends a word that must match whole.
const MEETINGS: &[&str] = &[
    "meeting",
    "call=",
    "calls=",
    "zoom",
    "standup",
    "stand=",
    "sync=",
    "syncs=",
    "retro",
    "встреч",
    "созвон",
    "звонк",
    "звонок",
    "совещани",
    "планерк",
    "митинг",
    "ретро",
    "стендап",
    "interview",
    "интервью",
];
const DICTATIONS: &[&str] = &["dictat", "диктов", "надиктов", "продиктов", "диктант"];
const FILES: &[&str] = &[
    "file=",
    "files=",
    "recording",
    "transcript",
    "podcast",
    "lecture",
    "interview",
    "voice=",
    "файл",
    "записи=",
    "записях=",
    "записям=",
    "запись=",
    "записей=",
    "лекци",
    "интервью",
    "подкаст",
    "расшифровк",
];

/// What was said, decided, or asked to be summed up.
const SAID: &[&str] = &[
    "decid",
    "discuss",
    "talk",
    "said=",
    "say=",
    "agree",
    "mention",
    "promis",
    "summar",
    "recap",
    "topics=",
    "themes=",
    "action=",
    "deadline",
    "owner=",
    "who=",
    "dictated=",
    "ask=",
    "asked=",
    "compare",
    "changed=",
    "решил",
    "решени",
    "обсужд",
    "обсудил",
    "говорил",
    "сказал",
    "договор",
    "обещал",
    "упомина",
    "упомянул",
    "сведи",
    "сводк",
    "перескаж",
    "итог",
    "темы=",
    "тем=",
    "повторял",
    "кто=",
    "дедлайн",
    "задач",
    "надиктовал",
    "диктовал",
    "рассказ",
    "перенос",
    "сравни",
    "поменял",
    "изменил",
];

/// The person speaking of their own.
const PERSONAL: &[&str] = &[
    "my",
    "our",
    "we",
    "i",
    "me",
    "us",
    "мои",
    "моих",
    "моим",
    "мой",
    "моя",
    "мою",
    "моей",
    "наши",
    "наших",
    "нашим",
    "наш",
    "наша",
    "нашей",
    "мы",
    "я",
    "мне",
    "нами",
    "мной",
];

/// Asking how VibeHex works, which the guide answers.
const HOW: &[&str] = &[
    "how do",
    "how to",
    "how can",
    "can i",
    "could i",
    "can vibehex",
    "can it",
    "can assistant",
    "does vibehex",
    "does it",
    "is it possible",
    "where are",
    "where is",
    "where do",
    "turn on",
    "turn off",
    "set up",
    "settings",
    "stored=",
    "store=",
    "kept=",
    "delete",
    "retention",
    "why does",
    "как включ",
    "как настро",
    "как сдела",
    "как работа",
    "можно ли",
    "умеет ли",
    "где хран",
    "где наход",
    "где настро",
    "почему",
    "включить",
    "выключить",
    "настройк",
    "удалить",
    "хранятся",
];

/// Asking to find or bring back what was recorded, which keeps a question
/// with the recordings even when it also names a preference; `=` ends a word
/// that must match whole.
const RECALL: &[&str] = &[
    "find",
    "search",
    "show",
    "look= for=",
    "recall",
    "remind",
    "list=",
    "what= did=",
    "when= did=",
    "who= did=",
    "найди",
    "найти",
    "поищи",
    "покажи",
    "вспомни",
    "напомни",
    "перечисли",
    "что= я=",
    "что= мы=",
    "о= чем= я=",
    "о= чем= мы=",
    "когда= я=",
    "когда= мы=",
];

/// Saying how they want VibeHex to work for them: a setup question the
/// guide answers, whichever recordings it names.
const SETUP: &[&str] = &[
    "важн",
    "удобств",
    "удобн",
    "предпочит",
    "распозна",
    "точност",
    "настро",
    "под меня",
    "important",
    "matters",
    "prefer",
    "accura",
    "recogni",
    "convenien",
];

/// The time a question names, `[since, until)`.
fn when(words: &[String], now_ms: u64) -> Option<(u64, u64)> {
    let today = Day::of(now_ms);
    let day = |offset: i64| span(today.plus(-offset), today.plus(-offset + 1), now_ms);
    for (index, word) in words.iter().enumerate() {
        let next = words.get(index + 1).map(String::as_str).unwrap_or("");
        let before = index
            .checked_sub(1)
            .and_then(|i| words.get(i))
            .map(String::as_str)
            .unwrap_or("");
        match word.as_str() {
            "today" | "сегодня" | "сегодняшн" => return day(0),
            "yesterday" | "вчера" => return day(1),
            "позавчера" => return day(2),
            _ => {}
        }
        if starts(word, "сегодняшн") {
            return day(0);
        }
        if starts(word, "вчерашн") {
            return day(1);
        }
        let this = matches!(
            word.as_str(),
            "this" | "этой" | "эту" | "этом" | "эта" | "этот"
        );
        let last = matches!(
            word.as_str(),
            "last"
                | "past"
                | "previous"
                | "прошлой"
                | "прошлую"
                | "прошлом"
                | "прошлый"
                | "прошлая"
                | "предыдущей"
        );
        if (this || last) && (next == "week" || starts(next, "недел")) {
            let monday = today.plus(-today.weekday());
            return if this {
                span(monday, today.plus(1), now_ms)
            } else {
                span(monday.plus(-7), monday, now_ms)
            };
        }
        if (this || last) && (next == "month" || starts(next, "месяц")) {
            let first = Day::new(today.year, today.month, 1);
            return if this {
                span(first, today.plus(1), now_ms)
            } else {
                let previous = first.plus(-1);
                span(Day::new(previous.year, previous.month, 1), first, now_ms)
            };
        }
        // "last 7 days", "за последние 3 дня".
        if let Some(count) = number(word)
            && (next.starts_with("day") || starts(next, "дн") || starts(next, "день"))
            && (last || matches!(before, "last" | "past") || starts(before, "последн"))
        {
            return span(today.plus(-(count as i64) + 1), today.plus(1), now_ms);
        }
        if let Some(weekday) = weekday(word) {
            let back = (today.weekday() - weekday).rem_euclid(7);
            let back = if back == 0 && matches!(before, "last" | "прошлый" | "прошлую")
            {
                7
            } else {
                back
            };
            return day(back);
        }
        if let Some(month) = month(word, before) {
            // "18 ноября", "November 18": one day; else the whole month.
            let date = number(before)
                .or_else(|| number(next))
                .filter(|day| (1..=31).contains(day));
            let year = if month > today.month {
                today.year - 1
            } else {
                today.year
            };
            if let Some(date) = date {
                let start = Day::new(year, month, date as u32);
                return span(start, start.plus(1), now_ms);
            }
            let first = Day::new(year, month, 1);
            let next_month = Day::new(year, month, 28).plus(4);
            return span(
                first,
                Day::new(next_month.year, next_month.month, 1),
                now_ms,
            );
        }
    }
    None
}

/// How many "the last few" means when no number is given.
const FEW: usize = 5;

/// "The last five meetings", "последних трёх встреч", "my last call",
/// "последние диктовки" (a few), "последнюю диктовку" (one).
fn latest(words: &[String]) -> Option<usize> {
    for (index, word) in words.iter().enumerate() {
        if !(word == "last" || word == "latest" || word == "recent" || starts(word, "последн"))
        {
            continue;
        }
        let rest = &words[index + 1..words.len().min(index + 4)];
        let names = |word: &String| {
            MEETINGS
                .iter()
                .chain(DICTATIONS)
                .chain(FILES)
                .any(|stem| starts(word, stem))
        };
        let Some(kind) = rest.iter().position(names) else {
            continue;
        };
        if let Some(count) = rest[..kind].iter().find_map(|word| number(word)) {
            return Some(count.clamp(1, 20));
        }
        if rest[..kind].iter().any(|word| word == "few") {
            return Some(FEW);
        }
        if rest[..kind].iter().any(|word| word == "couple") {
            return Some(2);
        }
        // A Russian adjective says it: «последние» a few, «последнюю» one.
        if PLURAL_LAST.contains(&word.as_str()) {
            return Some(FEW);
        }
        if starts(word, "последн") {
            return Some(1);
        }
        let noun = &rest[kind];
        let plural = noun.ends_with('s')
            || [
                "встреч=",
                "встречи=",
                "встречах=",
                "встречам=",
                "созвонов=",
                "созвоны=",
                "звонков=",
                "звонки=",
                "диктовок=",
                "диктовках=",
                "файлов=",
                "файлы=",
                "записей=",
            ]
            .iter()
            .any(|form| starts(noun, form));
        return Some(if plural { FEW } else { 1 });
    }
    None
}

/// «Последние» in its plural forms.
const PLURAL_LAST: &[&str] = &["последние", "последних", "последним", "последними"];

fn number(word: &str) -> Option<usize> {
    if let Ok(number) = word.parse::<usize>() {
        return Some(number);
    }
    const NUMBERS: &[(&[&str], usize)] = &[
        (&["one", "один", "одну", "одной", "одна"], 1),
        (&["two", "два", "две", "двух"], 2),
        (&["three", "три", "трех"], 3),
        (&["four", "четыре", "четырех"], 4),
        (&["five", "пять", "пяти"], 5),
        (&["six", "шесть", "шести"], 6),
        (&["seven", "семь", "семи"], 7),
        (&["eight", "восемь", "восьми"], 8),
        (&["nine", "девять", "девяти"], 9),
        (&["ten", "десять", "десяти"], 10),
    ];
    NUMBERS
        .iter()
        .find(|(names, _)| names.contains(&word))
        .map(|(_, number)| *number)
}

/// Monday is 0.
fn weekday(word: &str) -> Option<i64> {
    const DAYS: [(&str, &[&str]); 7] = [
        ("monday", &["понедельник"]),
        ("tuesday", &["вторник"]),
        ("wednesday", &["среда=", "среду=", "среды=", "среде="]),
        ("thursday", &["четверг"]),
        ("friday", &["пятниц"]),
        ("saturday", &["суббот"]),
        ("sunday", &["воскресень"]),
    ];
    DAYS.iter()
        .position(|(english, russian)| {
            word == *english || russian.iter().any(|stem| starts(word, stem))
        })
        .map(|day| day as i64)
}

/// A month's number. English `may` counts only after "in" or "on".
fn month(word: &str, before: &str) -> Option<u32> {
    const MONTHS: [(&str, &str); 12] = [
        ("january", "январ"),
        ("february", "феврал"),
        ("march", "март"),
        ("april", "апрел"),
        ("may", "ма"),
        ("june", "июн"),
        ("july", "июл"),
        ("august", "август"),
        ("september", "сентябр"),
        ("october", "октябр"),
        ("november", "ноябр"),
        ("december", "декабр"),
    ];
    MONTHS
        .iter()
        .position(|(english, russian)| {
            if *english == "may" {
                (word == "may" && matches!(before, "in" | "on" | "since"))
                    || matches!(word, "май" | "мая" | "мае")
            } else {
                // `nov`, `ноября`.
                word == *english
                    || (word.len() == 3 && english.starts_with(word))
                    || word.starts_with(russian)
            }
        })
        .map(|month| month as u32 + 1)
}

/// The words worth searching for.
fn terms(words: &[String]) -> Vec<String> {
    let mut terms = Vec::new();
    for word in words {
        let skip = word.chars().count() < 3
            || number(word).is_some()
            || weekday(word).is_some()
            || month(word, "in").is_some()
            || STOP.contains(&word.as_str())
            || PERSONAL.contains(&word.as_str())
            || MEETINGS
                .iter()
                .chain(DICTATIONS)
                .chain(FILES)
                .chain(SAID)
                .chain(TIMES)
                .any(|stem| starts(word, stem));
        if skip {
            continue;
        }
        for term in stems(word) {
            if !terms.contains(&term) {
                terms.push(term);
            }
        }
    }
    terms
}

/// A Russian word cut to a stem the search's prefix match extends
/// (`бюджету` → `бюджет`); a short one also as its likely nominative
/// (`анной` → `анна`). Other words as they are.
fn stems(word: &str) -> Vec<String> {
    if !word.chars().any(|c| ('а'..='я').contains(&c)) {
        return vec![word.to_owned()];
    }
    const ENDINGS: &[&str] = &[
        "ами", "ями", "ого", "его", "ому", "ему", "ыми", "ими", "ах", "ях", "ам", "ям", "ов", "ев",
        "ой", "ей", "ом", "ем", "ую", "юю", "ая", "яя", "ые", "ие", "ый", "ий", "ых", "их", "а",
        "я", "у", "ю", "ы", "и", "е", "о",
    ];
    let Some(ending) = ENDINGS.iter().find(|ending| word.ends_with(*ending)) else {
        return vec![word.to_owned()];
    };
    let stem = &word[..word.len() - ending.len()];
    if stem.chars().count() >= 4 {
        vec![stem.to_owned()]
    } else {
        vec![word.to_owned(), format!("{stem}а")]
    }
}

/// Words that say when, not what.
const TIMES: &[&str] = &[
    "today",
    "yesterday",
    "week",
    "month",
    "last=",
    "past=",
    "this=",
    "previous",
    "recent",
    "latest",
    "days=",
    "сегодня",
    "вчера",
    "позавчера",
    "недел",
    "месяц",
    "прошл",
    "последн",
    "недавн",
    "дней=",
    "дня=",
    "день=",
];

#[cfg(test)]
mod tests;
