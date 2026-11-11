use std::path::PathBuf;

use super::*;
use crate::assistant::intent;
use crate::assistant::prompt::{Reach, Room, build};
use crate::assistant::setup::Stage;

/// The eval's recordings: ten meetings in October and November 2026, three
/// Files texts, fifteen dictations (ten in November naming VibeHex, half
/// of them as speech recognition writes it in Cyrillic, or not at all).
fn fixtures() -> Sources {
    Sources::at(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assistant/recordings"),
    )
}

/// Friday 27 November 2026, noon local.
fn now() -> u64 {
    crate::meeting::query::parse_time("2026-11-27", 0, false).unwrap() + 12 * 3_600_000
}

fn allowed() -> Allowed {
    Allowed::from_settings(&AssistantSettings {
        read_recordings: Some(true),
        ..AssistantSettings::default()
    })
    .unwrap()
}

fn found(question: &str, budget: Budget) -> Found {
    let intent = intent::read(question, now()).unwrap_or_else(|| panic!("{question}"));
    search(
        &allowed(),
        &fixtures(),
        &intent,
        &[],
        budget,
        now(),
        &mut |_| {},
    )
}

/// The recordings `question` reads, as `meeting:<slug>`, `file:<slug>`, or
/// `dictation:<id>`; None when it doesn't reach them.
fn read(question: &str) -> Option<Vec<String>> {
    intent::read(question, now())?;
    let mut ids: Vec<String> = Vec::new();
    for excerpt in found(question, Budget::LARGE).excerpts {
        let id = match excerpt.kind {
            Kind::Meeting | Kind::File => {
                let slug = excerpt
                    .id
                    .split_once('-')
                    .map_or(excerpt.id.as_str(), |(_, slug)| slug);
                let kind = if excerpt.kind == Kind::File {
                    "file"
                } else {
                    "meeting"
                };
                format!("{kind}:{slug}")
            }
            Kind::Dictation => format!("dictation:{}", excerpt.id),
        };
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Some(ids)
}

#[test]
fn only_a_yes_allows_reading() {
    let mut settings = AssistantSettings::default();
    assert!(Allowed::from_settings(&settings).is_none(), "not asked yet");
    settings.read_recordings = Some(false);
    assert!(Allowed::from_settings(&settings).is_none(), "said no");
    settings.read_recordings = Some(true);
    assert!(Allowed::from_settings(&settings).is_some());
}

#[test]
fn an_answer_cites_only_what_was_found_once_each() {
    let found = found(
        "Что мы решили по бюджету на встречах в ноябре?",
        Budget::SMALL,
    );
    assert!(found.excerpts.len() >= 2);
    let refs = found.cited("Лимит утвердили [2], до этого переносили [1][2]. [99]");
    assert_eq!(refs.iter().map(|r| r.n).collect::<Vec<_>>(), [2, 1]);
    assert_eq!(refs[0].id, found.excerpts[1].id);
    assert!(refs.iter().all(|r| r.lines.is_empty()), "no lines named");
}

/// The words of the item `excerpt` comes from, as its pane shows them: a
/// meeting's turns one per line, a Files Original's paragraphs.
fn item_text(excerpt: &Excerpt) -> String {
    let root = fixtures();
    match excerpt.kind {
        Kind::Meeting => {
            let scan = crate::meeting::query::Library::new(root.meetings)
                .scan()
                .unwrap();
            let meeting = scan.meetings.iter().find(|m| m.id == excerpt.id).unwrap();
            crate::meeting::coalesce_transcript(meeting.entries.iter().cloned())
                .into_iter()
                .map(|turn| turn.text)
                .collect::<Vec<_>>()
                .join("\n")
        }
        Kind::File => {
            let record = crate::files::store::list_in(&root.files)
                .into_iter()
                .find(|record| record.id == excerpt.id)
                .unwrap();
            crate::files::text::plain(&record.versions[0].segments)
        }
        Kind::Dictation => String::new(),
    }
}

/// The excerpt of `item` (its slug) and the id of its line holding `said`.
fn line_of<'a>(found: &'a Found, item: &str, said: &str) -> Option<(&'a Excerpt, u32)> {
    found
        .excerpts
        .iter()
        .filter(|excerpt| excerpt.id.ends_with(item))
        .find_map(|excerpt| {
            let index = excerpt
                .lines
                .iter()
                .position(|line| line.text.contains(said))?;
            Some((excerpt, index as u32 + 1))
        })
}

#[test]
fn cited_lines_map_to_their_times_and_words_with_word_times() {
    let found = found(
        "Что мы решили по бюджету на встречах в ноябре?",
        Budget::LARGE,
    );
    // Budget Q1 keeps Parakeet's word times: the passage is two sentences,
    // the second the one that says what was decided.
    let (excerpt, line) = line_of(&found, "budget-q1", "две десятых").unwrap();
    assert_eq!(line, 2, "{:?}", excerpt.lines);
    let n = excerpt.n;
    let refs = found.cited(&format!("Утвердили 1,2 млн [{n}.{line}]."));
    let lines = &refs[0].lines;
    assert_eq!(lines.len(), 1);
    assert_eq!((lines[0].first, lines[0].last), (2, 2));
    assert_eq!((lines[0].start_ms, lines[0].end_ms), (3_000, 8_500));
    let text = item_text(excerpt);
    assert_eq!(
        &text[lines[0].at.clone()],
        "Лимит на первый квартал — один и две десятых миллиона рублей."
    );
    // A run of lines spans from the first's first word to the last's end.
    let refs = found.cited(&format!("По бюджету [{n}.1-{n}.2]."));
    let run = &refs[0].lines[0];
    assert_eq!((run.start_ms, run.end_ms), (0, 8_500));
    assert!(text[run.at.clone()].starts_with("Итак, по бюджету. Лимит"));
}

#[test]
fn a_files_line_starts_at_its_first_timed_word() {
    let found = found(
        "What did the lecture say about the learning rate?",
        Budget::LARGE,
    );
    let (excerpt, line) = line_of(&found, "lecture", "step size").unwrap();
    let refs = found.cited(&format!("It sets the step [{}.{line}].", excerpt.n));
    let lines = &refs[0].lines[0];
    assert_eq!((lines.start_ms, lines.end_ms), (6_300, 11_000));
    assert_eq!(
        &item_text(excerpt)[lines.at.clone()],
        "The learning rate controls the step size."
    );
}

#[test]
fn cited_lines_map_to_their_segments_without_word_times() {
    let found = found("Когда на встречах переносили релиз?", Budget::LARGE);
    let (excerpt, line) = line_of(&found, "planerka", "двигаем на декабрь").unwrap();
    let refs = found.cited(&format!("На декабрь [{}.{line}].", excerpt.n));
    let lines = &refs[0].lines[0];
    assert_eq!((lines.start_ms, lines.end_ms), (69_000, 75_340));
    assert_eq!(
        &item_text(excerpt)[lines.at.clone()],
        "Релиз двигаем на декабрь."
    );
}

/// Ids an excerpt hasn't are dropped without a word: the place then shows
/// only its passage, and an unknown excerpt isn't a place at all.
#[test]
fn unknown_lines_fall_back_to_the_passage() {
    let found = found("Когда на встречах переносили релиз?", Budget::LARGE);
    let (excerpt, _) = line_of(&found, "planerka", "двигаем на декабрь").unwrap();
    let n = excerpt.n;
    let refs = found.cited(&format!(
        "A [{n}.0], b [{n}.99], c [{n}.1-{n}.99], d [99.1]."
    ));
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].n, n);
    assert!(refs[0].lines.is_empty(), "{:?}", refs[0].lines);
    assert_eq!(refs[0].offset_ms, excerpt.offset_ms);
    // Each of the answer's marks for an excerpt keeps its own lines.
    let refs = found.cited(&format!("A [{n}.1]. B [{n}.2][{n}.1]. C [{n}]."));
    let marks: Vec<(u32, u32)> = refs[0].lines.iter().map(|l| (l.mark, l.first)).collect();
    assert_eq!(marks, [(0, 1), (1, 2), (1, 1)]);
}

#[test]
fn the_search_reports_its_steps_as_it_goes() {
    let intent = intent::read("Summarize my calls with Anna last week", now()).unwrap();
    let mut seen = Vec::new();
    let found = search(
        &allowed(),
        &fixtures(),
        &intent,
        &[],
        Budget::LARGE,
        now(),
        &mut |step| seen.push(step.clone()),
    );
    assert_eq!(seen, found.steps);
    assert_eq!(
        seen[0],
        Step::Meetings {
            searched: 3,
            found: 2
        },
        "three meetings last week, two with Anna"
    );
    assert!(matches!(seen[1], Step::Read { notes: 2, .. }));
}

/// The dictations `question` reads, by id, in the order read.
fn dictations_read(question: &str) -> (Vec<String>, Vec<Step>) {
    let found = found(question, Budget::LARGE);
    let ids = found
        .excerpts
        .iter()
        .filter(|excerpt| excerpt.kind == Kind::Dictation)
        .map(|excerpt| excerpt.id.clone())
        .collect();
    (ids, found.steps)
}

#[test]
fn the_last_few_that_match_are_the_newest_matches_of_all() {
    let (ids, steps) =
        dictations_read("А можешь найти последние мои диктовки, где я говорил о VibeHex?");
    assert_eq!(
        ids,
        ["15", "11", "10", "9", "8"],
        "the newest five of the ten that say it, however written"
    );
    assert_eq!(
        steps,
        [Step::Dictations {
            searched: 15,
            found: 5
        }]
    );
    // Without words to search for, the last few are simply the newest.
    let (ids, _) = dictations_read("Покажи мои последние диктовки");
    assert_eq!(ids, ["2", "1", "15", "3", "14"]);
    let (ids, _) = dictations_read("Покажи мою последнюю диктовку");
    assert_eq!(ids, ["2"]);
    // Meetings alike: the newest that name OpenRouter, not the newest five.
    assert_eq!(
        read("Что решили про OpenRouter на последних встречах?").unwrap(),
        ["meeting:retro", "meeting:budget-q1"]
    );
}

/// Speech recognition writes an English name in Cyrillic: a search for
/// VibeHex finds «вайп хекс», «вайбхекс», «вайпхекс», «вайб хексе», and
/// "vibe hex" too, and nothing else.
#[test]
fn a_name_is_found_however_speech_recognition_wrote_it() {
    let (mut ids, _) = dictations_read("Найди мои диктовки, где я говорил про VibeHex");
    ids.sort_by_key(|id| id.parse::<u32>().unwrap());
    assert_eq!(ids, ["6", "7", "8", "9", "10", "11", "15"]);
    // Asked in Cyrillic, with the person's Vocabulary, the same ones.
    let vocabulary = [crate::dictation::vocabulary::VocabularyEntry {
        term: "VibeHex".into(),
        sounds_like: vec!["вайб хекс".into()],
    }];
    let intent = intent::read("Найди мои диктовки, где я говорил про вайб хекс", now()).unwrap();
    let found = search(
        &allowed(),
        &fixtures(),
        &intent,
        &vocabulary,
        Budget::LARGE,
        now(),
        &mut |_| {},
    );
    let mut ids: Vec<u32> = found
        .excerpts
        .iter()
        .map(|e| e.id.parse().unwrap())
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, [6, 7, 8, 9, 10, 11, 15]);
}

#[test]
fn a_small_model_reads_within_its_budget() {
    let found = found(
        "Какие темы повторялись на последних пяти встречах?",
        Budget::SMALL,
    );
    let chars: usize = found.excerpts.iter().map(|e| e.text.len()).sum();
    assert!(chars <= Budget::SMALL.chars, "{chars}");
    assert!(
        found
            .excerpts
            .iter()
            .enumerate()
            .all(|(i, e)| e.n == i as u32 + 1),
        "numbered from 1 in order"
    );
}

/// No personal text reaches the prompt unless the person allowed it: the
/// guide and "answer without recordings" carry none of it.
#[test]
fn no_recording_enters_the_prompt_without_consent() {
    let question = "Что мы решили по бюджету на встречах в ноябре?";
    let found = found(question, Budget::LARGE);
    let private: Vec<&str> = found
        .excerpts
        .iter()
        .flat_map(|e| match e.lines.as_slice() {
            [] => vec![e.text.as_str()],
            lines => lines.iter().map(|line| line.text.as_str()).collect(),
        })
        .flat_map(|text| text.split(['.', ',', ':']))
        .map(str::trim)
        .filter(|piece| piece.chars().count() > 20)
        .collect();
    assert!(private.len() > 3);
    for reach in [Reach::Guide, Reach::Declined] {
        let prompt = build(question, &[], Room::Large, reach, None, &Stage::None);
        let all = format!("{}\n{}", prompt.instructions, prompt.input);
        assert!(!all.contains("<recordings"));
        for piece in &private {
            assert!(!all.contains(piece), "{piece:?} leaked");
        }
    }
    let prompt = build(
        question,
        &[],
        Room::Large,
        Reach::Recordings(&found, now()),
        None,
        &Stage::None,
    );
    assert!(prompt.input.contains("<recordings"));
    assert!(private.iter().all(|piece| prompt.input.contains(piece)));
}

/// Questions about recordings and what they should read. Recall over all
/// expected recordings must stay at 90% or more.
const ASKED: &[(&str, &[&str])] = &[
    (
        "Что мы решили по бюджету на встречах в ноябре?",
        &["meeting:budget-q1", "meeting:hiring", "meeting:planerka"],
    ),
    (
        "Summarize my calls with Anna last week",
        &["meeting:sync-anna", "meeting:pricing-anna"],
    ),
    (
        "What did I dictate about the release yesterday?",
        &["dictation:1", "dictation:2"],
    ),
    (
        "Какие темы повторялись на последних пяти встречах?",
        &[
            "meeting:standup",
            "meeting:retro",
            "meeting:design",
            "meeting:pricing-anna",
            "meeting:budget-q1",
        ],
    ),
    (
        "What did we decide about pricing with Anna?",
        &["meeting:pricing-anna"],
    ),
    (
        "Когда на встречах переносили релиз?",
        &["meeting:planerka", "meeting:standup"],
    ),
    (
        "What did Acme ask for on the sales call?",
        &["meeting:acme"],
    ),
    (
        "Что кандидат рассказал на интервью про Go?",
        &["meeting:interview"],
    ),
    ("Summarize the design review meeting", &["meeting:design"]),
    (
        "Кто ведёт найм бэкенда по итогам встречи?",
        &["meeting:hiring"],
    ),
    (
        "Что решили про OpenRouter на встречах?",
        &["meeting:retro", "meeting:budget-q1"],
    ),
    (
        "What was said at the standup yesterday?",
        &["meeting:standup"],
    ),
    ("О чём была встреча в среду?", &["meeting:retro"]),
    (
        "What did we discuss in the meeting on November 24?",
        &["meeting:design"],
    ),
    (
        "Сведи встречи за последние 3 дня",
        &["meeting:retro", "meeting:standup"],
    ),
    ("What did I dictate to Oleg?", &["dictation:2"]),
    ("Что я диктовал про онбординг?", &["dictation:4"]),
    (
        "Find my dictation about the Okta contract",
        &["dictation:5"],
    ),
    ("What's in my budget draft file?", &["file:budget"]),
    (
        "What did the lecture say about the learning rate?",
        &["file:lecture"],
    ),
    (
        "Summarize the podcast I transcribed in October",
        &["file:podcast"],
    ),
    (
        "When did we decide to move the release in meetings last week?",
        &["meeting:sync-anna"],
    ),
    (
        "Какие встречи были на прошлой неделе?",
        &[
            "meeting:sync-anna",
            "meeting:budget-q1",
            "meeting:pricing-anna",
        ],
    ),
    ("What did we decide in my last call?", &["meeting:standup"]),
    ("Что я надиктовал Ане про OpenRouter?", &["dictation:3"]),
    (
        "Найди мои диктовки, где я говорил про онбординг",
        &["dictation:4"],
    ),
    ("Что я диктовал вчера?", &["dictation:1", "dictation:2"]),
    (
        "Сравни встречи Планёрка и Бюджет Q1: что поменялось?",
        &["meeting:planerka", "meeting:budget-q1"],
    ),
];

/// Questions and the line each should cite: the recording, then words of
/// the line that says it, which must be offered as a line of its own and
/// map back to exactly those words in the transcript. Word-timed (Budget
/// Q1, the lecture) and segment-only transcripts both.
const LINES: &[(&str, &str, &str)] = &[
    (
        "Что мы решили по бюджету на встречах в ноябре?",
        "budget-q1",
        "две десятых миллиона",
    ),
    (
        "Что мы решили по бюджету на встречах в ноябре?",
        "planerka",
        "перенесём на следующую неделю",
    ),
    (
        "Когда на встречах переносили релиз?",
        "planerka",
        "двигаем на декабрь",
    ),
    (
        "What did we decide about pricing with Anna?",
        "pricing-anna",
        "starter plan at twelve dollars",
    ),
    (
        "What did Acme ask for on the sales call?",
        "acme",
        "single sign-on with Okta",
    ),
    (
        "Кто ведёт найм бэкенда по итогам встречи?",
        "hiring",
        "Найм ведёт Олег",
    ),
    (
        "What did the lecture say about the learning rate?",
        "lecture",
        "controls the step size",
    ),
    (
        "When did we decide to move the release in meetings last week?",
        "sync-anna",
        "December third",
    ),
];

#[test]
fn the_eval_offers_and_maps_the_line_each_question_cites() {
    let mut hit = 0;
    let mut misses = Vec::new();
    for (question, item, said) in LINES {
        let found = found(question, Budget::LARGE);
        let fold = |text: &str| text.to_lowercase();
        let Some((excerpt, line)) = found.excerpts.iter().find_map(|excerpt| {
            let index = excerpt
                .lines
                .iter()
                .position(|line| fold(&line.text).contains(&fold(said)))?;
            excerpt
                .id
                .ends_with(item)
                .then_some((excerpt, index as u32 + 1))
        }) else {
            misses.push(format!("{question}: no line of {item} says {said:?}"));
            continue;
        };
        let refs = found.cited(&format!("So [{}.{line}].", excerpt.n));
        let Some(cited) = refs.first().and_then(|r| r.lines.first()) else {
            misses.push(format!("{question}: [{}.{line}] cited nothing", excerpt.n));
            continue;
        };
        let text = item_text(excerpt);
        let shown = text.get(cited.at.clone()).unwrap_or_default();
        let offered = &excerpt.lines[line as usize - 1];
        if fold(shown).contains(&fold(said))
            && shown == offered.text
            && (cited.start_ms, cited.end_ms) == (offered.start_ms, offered.end_ms)
        {
            hit += 1;
        } else {
            misses.push(format!("{question}: [{}.{line}] lit {shown:?}", excerpt.n));
        }
    }
    let recall = f64::from(hit) / LINES.len() as f64;
    eprintln!("line recall {recall:.2} ({hit}/{})", LINES.len());
    for miss in &misses {
        eprintln!("missed {miss}");
    }
    assert!(misses.is_empty(), "{misses:?}");
}

/// Questions about VibeHex: none may bring up the consent card.
const GUIDE: &[&str] = &[
    "How do I turn on meeting notes?",
    "Can VibeHex transcribe a video file?",
    "Как включить запись встреч?",
    "What does the Assistant send to the model?",
    "How do I delete my dictation history?",
    "Which models keep word times?",
    "Как поменять горячую клавишу диктовки?",
    "Does Hex work offline?",
    "Где хранятся мои записи?",
    "Can Zoom calls record by themselves, without asking?",
    "Which speech model is best for Russian?",
    "Помоги мне настроить VibeHex под меня",
    "macOS на русском языке, но вставлять английские символы. Мне важно удобство диктовки \
     технических терминов, чтобы они нормально распознавались. Но точность тоже важна.",
];

#[test]
fn the_eval_finds_what_each_question_is_about() {
    let mut expected = 0;
    let mut hit = 0;
    let mut misses = Vec::new();
    for (question, want) in ASKED {
        let got = read(question).unwrap_or_default();
        for id in *want {
            expected += 1;
            if got.iter().any(|found| found == id) {
                hit += 1;
            } else {
                misses.push(format!("{question}: {id} (read {got:?})"));
            }
        }
    }
    let recall = f64::from(hit) / f64::from(expected);
    eprintln!("recall {recall:.2} ({hit}/{expected})");
    for miss in &misses {
        eprintln!("missed {miss}");
    }
    assert!(recall >= 0.9, "recall {recall:.2}");
    for question in GUIDE {
        assert_eq!(intent::read(question, now()), None, "{question} would ask");
    }
}

/// Follow-ups that only refer back: the earlier question, the follow-up,
/// and what it should read again.
const FOLLOW_UPS: &[(&str, &str, &[&str])] = &[
    (
        "А можешь найти последние мои диктовки, где я говорил о VibeHex?",
        "А можешь ответить также на последний вопрос, который я задал?",
        &["dictation:15", "dictation:11"],
    ),
    (
        "Summarize my calls with Anna last week",
        "and the other ones?",
        &["meeting:sync-anna", "meeting:pricing-anna"],
    ),
    (
        "Что мы решили по бюджету на встречах в ноябре?",
        "Подробнее",
        &["meeting:budget-q1"],
    ),
    (
        "What did I dictate to Oleg?",
        "Answer my last question",
        &["dictation:2"],
    ),
];

#[test]
fn the_eval_follows_a_follow_up_back() {
    for (asked, follow_up, want) in FOLLOW_UPS {
        assert_eq!(intent::read(follow_up, now()), None, "{follow_up} alone");
        let intent = intent::carried(follow_up, &[asked], now())
            .unwrap_or_else(|| panic!("{follow_up} after {asked}"));
        let found = search(
            &allowed(),
            &fixtures(),
            &intent,
            &[],
            Budget::LARGE,
            now(),
            &mut |_| {},
        );
        for id in *want {
            let (kind, slug) = id.split_once(':').unwrap();
            assert!(
                found.excerpts.iter().any(|excerpt| {
                    let kind_matches = match excerpt.kind {
                        Kind::Meeting => kind == "meeting",
                        Kind::File => kind == "file",
                        Kind::Dictation => kind == "dictation",
                    };
                    kind_matches
                        && (excerpt.id == slug || excerpt.id.ends_with(&format!("-{slug}")))
                }),
                "{follow_up} after {asked}: {id}"
            );
        }
    }
    // After a guide question, or asked first, a follow-up stays with the guide.
    for question in GUIDE {
        for follow_up in ["Answer my last question", "ещё", "Подробнее"] {
            assert_eq!(
                intent::carried(follow_up, &[question], now()),
                None,
                "{follow_up} after {question}"
            );
        }
        assert_eq!(
            intent::carried(question, &[FOLLOW_UPS[0].0], now()),
            None,
            "{question} after a recordings question"
        );
    }
}

/// A question in one meeting's Requests reads the others, never itself.
#[test]
fn a_search_from_one_recording_leaves_it_out() {
    let question = "Что мы решили по бюджету на встречах в ноябре?";
    let intent = intent::read(question, now()).unwrap();
    let all = found(question, Budget::LARGE);
    let budget = all
        .excerpts
        .iter()
        .find(|excerpt| excerpt.id.ends_with("budget-q1"))
        .expect("the budget meeting matches")
        .id
        .clone();
    let without = search(
        &allowed(),
        &fixtures().without(Kind::Meeting, &budget),
        &intent,
        &[],
        Budget::LARGE,
        now(),
        &mut |_| {},
    );
    assert!(!without.excerpts.is_empty());
    assert!(without.excerpts.iter().all(|excerpt| excerpt.id != budget));
    // Only a meeting's own id is left out: a file of that id would stay.
    let file = fixtures().without(Kind::File, &budget);
    assert_eq!(file.except(Kind::Meeting), None);
}

/// "На прошлых встречах", asked of the design review: the meetings before
/// it that match, never the review itself.
#[test]
fn the_previous_meetings_are_those_before_this_one() {
    let item = intent::beyond::Item {
        kind: Kind::Meeting,
        start_ms: 1_795_512_600_000,
        end_ms: 1_795_512_600_000 + 1_800_000,
        title: "Design review",
        text: "Design review",
    };
    let intent = intent::beyond::read(
        "Что мы решили по бюджету на прошлых встречах?",
        &item,
        now(),
    )
    .unwrap();
    assert!(intent.sure && intent.kinds.meetings && !intent.kinds.dictations);
    let found = search(
        &allowed(),
        &fixtures().without(Kind::Meeting, "1795512600000-design"),
        &intent,
        &[],
        Budget::LARGE,
        now(),
        &mut |_| {},
    );
    assert!(
        found
            .excerpts
            .iter()
            .any(|excerpt| excerpt.id.ends_with("budget-q1"))
    );
    assert!(found.excerpts.iter().all(|excerpt| {
        excerpt.kind == Kind::Meeting
            && excerpt.at_ms < item.start_ms
            && !excerpt.id.ends_with("design")
    }));
}
