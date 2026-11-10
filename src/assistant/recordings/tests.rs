use std::path::PathBuf;

use super::*;
use crate::assistant::intent;
use crate::assistant::prompt::{Reach, Room, build};

/// The eval's recordings: ten meetings in October and November 2026, three
/// Files texts, five dictations.
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
    search(&allowed(), &fixtures(), &intent, budget, now(), &mut |_| {})
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
fn citations_are_read_in_every_form() {
    assert_eq!(
        citations("Budget [1], hiring [2, 4] and [3][5]."),
        [1, 2, 4, 3, 5]
    );
    assert_eq!(citations("[see below] [x] [] [12"), Vec::<u32>::new());
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
}

#[test]
fn the_search_reports_its_steps_as_it_goes() {
    let intent = intent::read("Summarize my calls with Anna last week", now()).unwrap();
    let mut seen = Vec::new();
    let found = search(
        &allowed(),
        &fixtures(),
        &intent,
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
        .flat_map(|e| e.text.split(['.', ',', ':']))
        .map(str::trim)
        .filter(|piece| piece.chars().count() > 20)
        .collect();
    assert!(private.len() > 3);
    for reach in [Reach::Guide, Reach::Declined] {
        let prompt = build(question, &[], Room::Large, reach);
        let all = format!("{}\n{}", prompt.instructions, prompt.input);
        assert!(!all.contains("<recordings"));
        for piece in &private {
            assert!(!all.contains(piece), "{piece:?} leaked");
        }
    }
    let prompt = build(question, &[], Room::Large, Reach::Recordings(&found, now()));
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
        "Сравни встречи Планёрка и Бюджет Q1: что поменялось?",
        &["meeting:planerka", "meeting:budget-q1"],
    ),
];

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
