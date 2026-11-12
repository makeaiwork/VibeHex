//! The scenarios about answers: what the model is told of the install,
//! which cards an answer may make, recordings, the web, citations.

use serde_json::json;

use super::checks::{
    answer_in_russian, answer_names, card_for, check, ensure, live, marks, names_before,
    no_card_for, no_cards, no_labs_card, offline, routed, sent_has, sent_lacks, web_off,
};
use super::fixtures::{folder, newcomer, owner, owner_with_meetings};
use super::harness::{Check, Fixture, Said, Scenario, Step};
use crate::assistant::changes::CardKind;
use crate::assistant::intent::Route;
use crate::assistant::recordings::{Kind, Step as Read};

pub fn message(text: &'static str, canned: &[&'static str], checks: Vec<Check>) -> Step {
    Step {
        said: Said::Message(text),
        web: false,
        canned: canned.to_vec(),
        checks,
    }
}

/// [`message`] in a conversation whose Web is on.
fn online(text: &'static str, canned: &[&'static str], checks: Vec<Check>) -> Step {
    Step {
        web: true,
        ..message(text, canned, checks)
    }
}

fn guide() -> Check {
    routed(|route| *route == Route::Guide, "the guide")
}

fn recordings() -> Check {
    check("routed to the recordings and read", |outcome| {
        ensure(
            matches!(outcome.route, Some(Route::Recordings(_))) && !outcome.consent,
            || format!("{:?}, consent {}", outcome.route, outcome.consent),
        )
    })
}

/// The dictations a search found.
fn dictations_found(steps: &[Read]) -> usize {
    steps
        .iter()
        .map(|step| match step {
            Read::Dictations { found, .. } => *found,
            _ => 0,
        })
        .sum()
}

fn several_dictations() -> Check {
    check("more than one dictation found", |outcome| {
        let found = dictations_found(&outcome.turn.steps);
        ensure(found > 1, || {
            format!("found {found}: {:?}", outcome.turn.steps)
        })
    })
}

/// The ids of the dictations a fixture's History holds, read as JSON.
fn dictation_ids(root: &std::path::Path) -> Vec<String> {
    std::fs::read(root.join("history.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|history| history["entries"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|entry| entry["id"].to_string())
        .collect()
}

/// Every `[n]` an answer carries is an excerpt it was given, and every
/// excerpt cited is a recording the fixture holds.
fn citations_resolve() -> Check {
    check(
        "every citation resolves to a fixture recording",
        |outcome| {
            let turn = &outcome.turn;
            let root = outcome.fixture.recordings.clone().unwrap_or_default();
            for mark in marks(&turn.answer) {
                ensure(turn.refs.iter().any(|known| known.n == mark), || {
                    format!("[{mark}] names no excerpt of {:?}", turn.refs)
                })?;
            }
            for cited in &turn.refs {
                let held = match cited.kind {
                    Kind::Meeting => root.join("meetings").join(&cited.id).is_dir(),
                    Kind::File => {
                        root.join("files")
                            .join(format!("{}.json", cited.id))
                            .is_file()
                            || root.join("files").join(&cited.id).exists()
                    }
                    Kind::Dictation => dictation_ids(&root).contains(&cited.id),
                };
                ensure(held, || format!("{cited:?} isn't in the fixture"))?;
                // The lines a mark names are lines the excerpt has.
                for lines in &cited.lines {
                    ensure(
                        lines.first <= lines.last && lines.start_ms <= lines.end_ms,
                        || format!("{lines:?}"),
                    )?;
                }
            }
            Ok(())
        },
    )
}

/// `fixture`'s notes written by Codex while Summaries still says "On this
/// Mac".
fn codex_writer() -> Fixture {
    let mut fixture = owner();
    fixture.settings.meetings.summaries = crate::app_settings::MeetingSummaries::Mac;
    fixture
}

/// A person who dictates code in Russian on a fresh install.
fn coder() -> Fixture {
    let mut fixture = newcomer();
    fixture.settings.labs.russian_vibe_coding = true;
    fixture
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "01-models-downloaded",
            about: "Asked which speech models are downloaded, it said none were, and offered a card.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                "Какие модели распознавания у меня скачаны?",
                &[
                    "Скачаны две модели: **Orukeet**, она сейчас используется, и **Parakeet v3**.\n\n\
                   Sources: topics/speech-models",
                ],
                vec![
                    guide(),
                    sent_has(
                        "the models on disk",
                        &[
                            "Speech models downloaded: Orukeet (orukeet, in use), Parakeet v3 (parakeet_v3)\n",
                        ],
                    ),
                    sent_lacks(
                        "\"none\" for the models",
                        &["Speech models downloaded: none"],
                    ),
                    answer_names(&["Orukeet", "Parakeet v3"]),
                    no_cards(),
                    answer_in_russian(),
                ],
            )],
        },
        Scenario {
            id: "02-notes-writer",
            about: "With Codex writing meeting notes and Summaries on \"On this Mac\", it said notes \
                    are written on this Mac and offered a Written by card.",
            fixture: codex_writer,
            app: Vec::new(),
            steps: vec![message(
                "Кто у меня пишет заметки встреч?",
                &[
                    "Заметки пишет Codex.\n\n```hex-change\n\
                     {\"key\":\"meetings.notes.writer\",\"value\":\"codex:gpt-6.1-terra\",\"why\":\"пусть пишет Codex\"}\n\
                     ```\n\nSources: topics/meetings",
                    "Заметки встреч у вас пишет **Codex**; Apple Intelligence сейчас не используется.\n\n\
                     Sources: topics/meetings",
                ],
                vec![
                    guide(),
                    sent_has(
                        "the writer in use",
                        &["Meeting notes: written by Codex", "is not used now."],
                    ),
                    no_card_for("meetings.notes.writer"),
                    no_card_for("meetings.summaries"),
                    answer_names(&["Codex"]),
                    // Apple Intelligence may be named as the fallback, never first.
                    names_before(&["Codex"], &["Apple Intelligence", "на этом Mac"]),
                ],
            )],
        },
        Scenario {
            id: "03-no-change-to-itself",
            about: "Cards offered a setting's own value: the same model, the same engine under \
                    another id, a feature that was on.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                "Включи мне Files и поставь для файлов Codex, а модель распознавания Orukeet",
                &[
                    "Сделаю.\n\n```hex-change\n\
                     {\"key\":\"labs.files\",\"value\":true,\"why\":\"Files\"}\n\
                     {\"key\":\"files.cleanup_engine\",\"value\":\"codex\",\"why\":\"Codex\"}\n\
                     {\"key\":\"transcription.model\",\"value\":\"orukeet\",\"why\":\"Orukeet\"}\n\
                     ```\n\nSources: labs/files",
                    "Всё это уже так и настроено: **Files** включён, файлы чистит Codex, модель \
                     распознавания Orukeet.\n\nSources: labs/files",
                ],
                vec![
                    guide(),
                    // "No card changes a value to itself" holds every step of
                    // every scenario; here nothing else may be left either.
                    no_card_for("labs.files"),
                    no_card_for("files.cleanup_engine"),
                    no_card_for("transcription.model"),
                    answer_in_russian(),
                ],
            )],
        },
        Scenario {
            id: "04-no-labs-workaround",
            about: "Asked which models it has, it offered Labs › Downloaded models only as a way \
                    to look.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                "Which speech models do I have?",
                &[
                    "You have **Orukeet** (in use) and **Parakeet v3** downloaded. The full list is in \
                   **General › Speech model**.\n\nSources: topics/speech-models",
                ],
                vec![
                    guide(),
                    sent_has(
                        "the models on disk",
                        &["Speech models downloaded: Orukeet (orukeet, in use)"],
                    ),
                    sent_has(
                        "the rule against a card to look something up",
                        &["Never a card as a way around what you don't know"],
                    ),
                    answer_names(&["Orukeet", "Parakeet v3"]),
                    no_labs_card(),
                    no_cards(),
                    super::checks::answer_in_english(),
                ],
            )],
        },
        Scenario {
            id: "06-model-for-russian-code",
            about: "A person dictating code in Russian was told GigaAM is the best speech model.",
            fixture: coder,
            app: Vec::new(),
            steps: vec![message(
                "Я диктую по-русски, много кода и английских терминов. Какая модель распознавания лучше?",
                &[
                    "Для русской речи с английскими терминами лучше **Orukeet** или **Parakeet v3**: они \
                   пишут термины латиницей. GigaAM v3 хорош для чистого русского, но термины пишет \
                   кириллицей.\n\n```hex-change\n\
                   {\"key\":\"transcription.model\",\"options\":[\"orukeet\",\"parakeet_v3\"],\"suggest\":\"orukeet\",\"why\":\"термины латиницей\"}\n\
                   ```\n\nSources: topics/speech-models",
                ],
                vec![
                    guide(),
                    names_before(&["Orukeet", "Parakeet v3"], &["GigaAM"]),
                    check("no card suggests GigaAM", |outcome| {
                        let gigaam = |value: &serde_json::Value| {
                            value.as_str().is_some_and(|id| id.starts_with("gigaam"))
                        };
                        let suggested = outcome.turn.cards.iter().any(|card| match &card.kind {
                            CardKind::Choice {
                                options, suggested, ..
                            } => options.get(*suggested).is_some_and(gigaam),
                            _ => card
                                .items
                                .iter()
                                .any(|item| item.key == "transcription.model" && gigaam(&item.to)),
                        });
                        ensure(!suggested, || format!("{:?}", outcome.turn.cards))
                    }),
                    answer_in_russian(),
                ],
            )],
        },
        Scenario {
            id: "07-words-wrong",
            about: "Told some words come out wrong, it didn't start from Add to vocabulary, which \
                    was on, or name the person's shortcut for it.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                "Некоторые слова распознаются неправильно, что с этим делать?",
                &[
                    "У вас уже включено **Labs › Add to vocabulary from a selection**: выделите слово, \
                   которое вышло неправильно, и нажмите ⌥⇧A, оно попадёт в **General › Vocabulary**.\n\n\
                   Sources: labs/vocabulary-quick-add, topics/vocabulary",
                ],
                vec![
                    sent_has(
                        "the shortcut as its keycaps",
                        &["Add selection to vocabulary ⌥⇧A"],
                    ),
                    answer_names(&["⌥⇧A"]),
                    check("it leads with the person's shortcut", |outcome| {
                        // In the answer's first paragraph, before anything
                        // else is suggested.
                        let answer = outcome.turn.answer.trim();
                        let first = answer.split("\n\n").next().unwrap_or_default();
                        ensure(first.contains("⌥⇧A"), || answer.to_owned())
                    }),
                    // It is on already: never a card to turn it on.
                    no_card_for("labs.vocabulary_quick_add"),
                    answer_in_russian(),
                ],
            )],
        },
        Scenario {
            id: "08-ai-cleanup-instead-of-ai",
            about: "«Почему пишет AI cleanup вместо AI»: it said the prompt wasn't passed to it, and \
                    showed neither what was heard nor the Vocabulary entry behind it.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![online(
                "Почему пишет AI cleanup вместо AI?",
                &[
                    "Дело в словаре: запись **AI cleanup** со звучанием «эй ай» ловит слово, когда вы \
                   говорите просто «AI» [1]. Услышано было «эй ай», записано «AI cleanup».\n\n\
                   ```hex-vocab\n{\"term\":\"AI cleanup\",\"remove_sounds_like\":[\"эй ай\"]}\n```\n\n\
                   Sources: topics/vocabulary",
                ],
                vec![
                    recordings(),
                    sent_has("the cleanup instructions", &["<cleanup_instructions>"]),
                    sent_has(
                        "the Vocabulary entry that catches it",
                        &["<vocabulary total=", "AI cleanup", "эй ай"],
                    ),
                    sent_has("what was heard and written", &["Heard: ", "Wrote: "]),
                    check("it names the Vocabulary entry as the cause", |outcome| {
                        let edit = outcome
                            .turn
                            .cards
                            .iter()
                            .any(|card| card.kind == CardKind::VocabularyEdit);
                        ensure(edit || outcome.turn.answer.contains("эй ай"), || {
                            format!("{} cards: {:?}", outcome.turn.answer, outcome.turn.cards)
                        })
                    }),
                    offline("the entry's card changes that spelling", |outcome| {
                        let edit = outcome
                            .turn
                            .cards
                            .iter()
                            .any(|card| card.kind == CardKind::VocabularyEdit);
                        ensure(edit, || format!("{:?}", outcome.turn.cards))
                    }),
                    // It read dictations and Vocabulary entries.
                    web_off(),
                    answer_in_russian(),
                ],
            )],
        },
        Scenario {
            id: "09-dictations-about-vibehex",
            about: "«Последние диктовки про VibeHex» found one; said as «вайп хекс» found none; the \
                    follow-up left the recordings.",
            fixture: owner_with_meetings,
            app: Vec::new(),
            steps: vec![
                online(
                    "Покажи последние диктовки про VibeHex",
                    &[
                        "Вот последние диктовки про VibeHex: про иконку в доке [1], про поиск по истории \
                       [2] и про время записи [3].\n\nSources: none",
                    ],
                    vec![
                        recordings(),
                        several_dictations(),
                        check("more than one excerpt came back", |outcome| {
                            ensure(outcome.turn.refs.len() > 1, || {
                                format!("{:?}", outcome.turn.refs)
                            })
                        }),
                        citations_resolve(),
                        web_off(),
                    ],
                ),
                online(
                    "А в какой из них было про поиск?",
                    &["Про поиск по истории было в этой [1].\n\nSources: none"],
                    vec![recordings(), citations_resolve(), web_off()],
                ),
            ],
        },
        Scenario {
            id: "09b-said-as-heard",
            about: "The same search with the name as speech recognition writes it, «вайп хекс».",
            fixture: owner_with_meetings,
            app: Vec::new(),
            steps: vec![message(
                "Найди мои диктовки про вайп хекс",
                &[
                    "Нашёл несколько: про поиск по истории [1] и про горячую клавишу [2].\n\nSources: none",
                ],
                vec![recordings(), several_dictations(), citations_resolve()],
            )],
        },
        Scenario {
            id: "15-whats-new",
            about: "\"What's new\" listed things for the person as text, and offered cards for what \
                    was already on.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                "Что нового в VibeHex?",
                &[
                    "В этой версии появились Assistant и Files.\n\n```hex-change\n\
                     {\"key\":\"labs.files\",\"value\":true,\"why\":\"Files\"}\n\
                     {\"key\":\"labs.smart_spacing\",\"value\":true,\"why\":\"пробел после слова\"}\n\
                     ```\n\nSources: none",
                ],
                vec![
                    routed(
                        |route| matches!(route, Route::Releases(_)),
                        "the release notes",
                    ),
                    sent_has("the release notes", &["<releases running="]),
                    // On already: never a card ("no card changes a value to
                    // itself" holds it too).
                    no_card_for("labs.files"),
                    offline("what is off and fits comes as a card", |outcome| {
                        (card_for("labs.smart_spacing", Some(json!(true))).test)(outcome)
                    }),
                    live(
                        "what it suggests for the person comes as cards",
                        |outcome| {
                            // An answer that tells them to turn something on
                            // carries its card.
                            let answer = outcome.turn.answer.to_lowercase();
                            let tells = ["включите", "включить", "turn on", "попробуйте"]
                                .iter()
                                .any(|word| answer.contains(word));
                            ensure(!tells || !outcome.turn.cards.is_empty(), || {
                                outcome.turn.answer.clone()
                            })
                        },
                    ),
                    answer_in_russian(),
                ],
            )],
        },
        Scenario {
            id: "16-web",
            about: "The web was on for a question that carried Vocabulary entries, and a local \
                    server's address came back as a \"From the web\" chip.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![online(
                "Почему Ghostty пишется как «гости»? Посмотри мой словарь",
                &[
                    "В словаре у **Ghostty** есть звучание «гости», поэтому обычное слово «гости» \
                   тоже превращается в Ghostty.\n\nSources: topics/vocabulary",
                ],
                vec![
                    sent_has("the Vocabulary entry", &["<vocabulary total=", "Ghostty"]),
                    web_off(),
                ],
            )],
        },
        Scenario {
            id: "16b-local-address",
            about: "A local server's address in an answer is a setting's value, never a web source.",
            fixture: newcomer,
            app: Vec::new(),
            steps: vec![online(
                "How do I connect LM Studio for AI cleanup? Check their docs online.",
                &[
                    "Start LM Studio's server, then set **Output › AI cleanup › Your server** to \
                   `http://127.0.0.1:1234/v1` (see http://127.0.0.1:1234/v1 and \
                   [LM Studio's docs](https://lmstudio.ai/docs/local-server)).\n\n\
                   Sources: topics/ai-cleanup",
                ],
                vec![
                    offline("it was allowed to search", |outcome| {
                        ensure(outcome.sent.as_ref().is_some_and(|sent| sent.web), || {
                            "the web was off".into()
                        })
                    }),
                    offline("the public page is the web source", |outcome| {
                        let sources = &outcome.turn.web.sources;
                        ensure(
                            sources.iter().any(|url| url.contains("lmstudio.ai")),
                            || format!("{sources:?}"),
                        )
                    }),
                    check("no web source is a local address", |outcome| {
                        let local = |url: &String| {
                            ["127.0.0.1", "localhost", "192.168.", "10.0.", ".local"]
                                .iter()
                                .any(|part| url.contains(part))
                        };
                        let sources = &outcome.turn.web.sources;
                        ensure(!sources.iter().any(local), || format!("{sources:?}"))
                    }),
                ],
            )],
        },
        Scenario {
            id: "17-citations",
            about: "An answer from meetings cited excerpts that weren't there, or lines the place \
                    couldn't show.",
            fixture: owner_with_meetings,
            app: vec![super::checks::app(
                "the fixture has meetings to cite",
                |_| {
                    ensure(folder("recordings").join("meetings").is_dir(), || {
                        "no meetings fixture".into()
                    })
                },
            )],
            steps: vec![message(
                "О чём договорились на встречах в ноябре?",
                &[
                    "На планёрке договорились выпустить сборку в четверг [1.1], а на собеседовании \
                   обсудили следующий этап [2].\n\nSources: none",
                ],
                vec![
                    recordings(),
                    check("meetings were read", |outcome| {
                        let read =
                            outcome.turn.steps.iter().any(
                                |step| matches!(step, Read::Meetings { found, .. } if *found > 0),
                            );
                        ensure(read, || format!("{:?}", outcome.turn.steps))
                    }),
                    live("the answer cites what it read", |outcome| {
                        ensure(
                            !marks(&outcome.turn.answer).is_empty()
                                || !outcome.turn.refs.is_empty(),
                            || outcome.turn.answer.clone(),
                        )
                    }),
                    citations_resolve(),
                    answer_in_russian(),
                ],
            )],
        },
    ]
}
