//! The scenarios about guided setup: what it asks and plans on this
//! computer, a returning setup, the preset, and its language.
//!
//! Which stage follows which is the pane's (`rune_ui::assistant::ask`);
//! each step here builds its stage with the same core calls the pane
//! makes, and the app checks hold those calls themselves, with no model.

use serde_json::Value;

use super::answers::message;
use super::checks::{
    answer_in_english, answer_in_russian, app, asks_in_russian, check, ensure, live, offline,
    routed, russian, sent_has,
};
use super::fixtures::{self, newcomer};
use super::harness::{Check, Fixture, Said, Scenario, Step};
use super::stages::{
    FOLLOW_UNFIT, LOOK, PLAN_TERMS, PLAN_UNFIT, ROUND_ONE, candidates, earlier, finished,
    follow_stage, look_stage, own_model, plan_stage, returning,
};
use crate::assistant::changes::{CardKind, Item, vocab};
use crate::assistant::facts::State;
use crate::assistant::intent::Route;
use crate::assistant::setup::{self, Ask, Stage, follow, plan, preset, since};
use crate::control::settings;
use crate::desktop::i18n::tr;

const NOTCH: &str = "labs.notch_indicator";
const MOUSE: &str = "labs.mouse_shortcuts";

fn stage(
    build: fn(&Fixture) -> (String, Stage),
    canned: &[&'static str],
    checks: Vec<Check>,
) -> Step {
    Step {
        said: Said::Stage(build),
        web: false,
        canned: canned.to_vec(),
        checks,
    }
}

fn plan_items(cards: &[crate::assistant::changes::Card]) -> Vec<&Item> {
    cards
        .iter()
        .filter(|card| card.kind == CardKind::Plan)
        .flat_map(|card| &card.items)
        .collect()
}

/// A plan's Vocabulary rows: one term a row, checked only when sure.
fn term_rows(items: &[&Item], candidates: &[since::Candidate]) -> Result<usize, String> {
    let rows: Vec<&&Item> = items.iter().filter(|item| item.key == vocab::KEY).collect();
    for row in &rows {
        let terms = vocab::terms(&row.to);
        ensure(terms.len() == 1, || {
            format!("a row holds {} terms", terms.len())
        })?;
        let term = &terms[0].term;
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.term.eq_ignore_ascii_case(term))
        {
            ensure(row.included == candidate.sure, || {
                format!("{term}: checked {}, sure {}", row.included, candidate.sure)
            })?;
        }
    }
    Ok(rows.len())
}

/// Ordinary English words cleanup writes inside Russian speech.
const ORDINARY: [&str; 12] = [
    "to", "the", "and", "ok", "okay", "test", "fix", "file", "then", "also", "so", "done",
];

fn setup_route() -> Check {
    routed(|route| *route == Route::Setup, "setup")
}

fn list_items(answer: &str) -> usize {
    answer
        .lines()
        .filter(|line| line.trim_start().starts_with("- ") || line.trim_start().starts_with("* "))
        .count()
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "05-hardware",
            about: "On a Mac mini with a Magic Mouse, setup asked about the Notch indicator and \
                    planned Mouse shortcuts.",
            fixture: newcomer,
            app: vec![
                app(
                    "the app's follow-ups leave out what this Mac can't use",
                    |fixture| {
                        let asks = follow::candidates(
                            &fixtures::answers(),
                            &fixture.settings,
                            &fixture.state,
                        );
                        let unfit: Vec<&str> = asks
                            .iter()
                            .map(|ask| ask.id.as_str())
                            .filter(|id| [NOTCH, MOUSE].contains(id))
                            .collect();
                        ensure(unfit.is_empty() && !asks.is_empty(), || {
                            format!("{unfit:?} of {}", asks.len())
                        })
                    },
                ),
                app("the app's plan leaves them out", |fixture| {
                    let card = plan::with_terms(
                        Vec::new(),
                        &fixtures::answers(),
                        &[],
                        &fixture.settings,
                        &fixture.state,
                    );
                    let keys: Vec<String> = card
                        .iter()
                        .flat_map(|card| card.items.iter().chain(&card.kept))
                        .map(|item| item.key.clone())
                        .collect();
                    ensure(
                        !keys.is_empty() && !keys.iter().any(|key| key == NOTCH || key == MOUSE),
                        || format!("{keys:?}"),
                    )
                }),
                app(
                    "with the hardware unknown they may still be asked",
                    |fixture| {
                        let unknown = State {
                            hardware: Default::default(),
                            ..fixture.state.clone()
                        };
                        ensure(
                            !follow::unfit(NOTCH, &unknown.hardware)
                                && follow::unfit(NOTCH, &fixture.state.hardware),
                            || "unfit while unknown".into(),
                        )
                    },
                ),
            ],
            steps: vec![
                stage(
                    follow_stage,
                    &[FOLLOW_UNFIT],
                    vec![
                        sent_has(
                            "what this Mac can't use",
                            &["Not on this Mac: ", "Notch indicator"],
                        ),
                        offline("a follow-up that fits is still asked", |outcome| {
                            ensure(
                                outcome
                                    .turn
                                    .asks
                                    .iter()
                                    .any(|ask| ask.id == "labs.filler_words"),
                                || {
                                    format!(
                                        "{:?}",
                                        outcome
                                            .turn
                                            .asks
                                            .iter()
                                            .map(|ask| &ask.id)
                                            .collect::<Vec<_>>()
                                    )
                                },
                            )
                        }),
                        check("follow-ups were asked", |outcome| {
                            ensure(!outcome.turn.asks.is_empty(), || "none".into())
                        }),
                    ],
                ),
                stage(
                    plan_stage,
                    &[PLAN_UNFIT],
                    vec![
                        check("a plan came", |outcome| {
                            ensure(!plan_items(&outcome.turn.cards).is_empty(), || {
                                format!("{:?}", outcome.turn.cards)
                            })
                        }),
                        offline("the model's line that fits is in the plan", |outcome| {
                            ensure(
                                plan_items(&outcome.turn.cards)
                                    .iter()
                                    .any(|item| item.key == "labs.filler_words"),
                                || format!("{:?}", outcome.turn.cards),
                            )
                        }),
                    ],
                ),
            ],
        },
        Scenario {
            id: "10-which-labs",
            about: "«Какие Labs мне включить» was answered as a guide question instead of opening \
                    setup.",
            fixture: newcomer,
            app: vec![app("every way of asking it opens setup", |fixture| {
                for question in [
                    "Какие Labs мне включить?",
                    "какие лабс мне стоит включить",
                    "Which Labs should I turn on?",
                    "Настрой VibeHex под меня",
                ] {
                    let route = crate::assistant::intent::route(
                        question,
                        &[],
                        &fixture.settings,
                        fixture.now_ms,
                    );
                    ensure(route == Route::Setup, || format!("{question}: {route:?}"))?;
                }
                // A question about one feature stays with the guide.
                let route = crate::assistant::intent::route(
                    "Что делает Labs › Smart spacing?",
                    &[],
                    &fixture.settings,
                    fixture.now_ms,
                );
                ensure(route == Route::Guide, || format!("{route:?}"))
            })],
            steps: vec![message(
                "Какие Labs мне включить?",
                &[ROUND_ONE],
                vec![
                    setup_route(),
                    check("round one's four questions are asked", |outcome| {
                        let ids: Vec<&str> = outcome
                            .turn
                            .asks
                            .iter()
                            .map(|ask| ask.id.as_str())
                            .collect();
                        ensure(ids == ["use", "language", "cleanup", "notes"], || {
                            format!("{ids:?}")
                        })
                    }),
                    check("no card before the answers", |outcome| {
                        ensure(outcome.turn.cards.is_empty(), || {
                            format!("{:?}", outcome.turn.cards)
                        })
                    }),
                ],
            )],
        },
        Scenario {
            id: "11-returning-setup",
            about: "A returning setup asked again what was answered, opened with a long text, \
                    offered Vocabulary before the answers, put several terms on a row, checked \
                    unsure ones, and offered ordinary English words as terms.",
            fixture: returning,
            app: vec![
                app("what was answered before isn't asked again", |fixture| {
                    let answers = fixtures::answers();
                    let answered: Vec<&str> = fixture
                        .state
                        .answered
                        .iter()
                        .map(|ask| ask.id.as_str())
                        .collect();
                    ensure(answered.len() == 2, || format!("answered: {answered:?}"))?;
                    let asked = follow::candidates(&answers, &fixture.settings, &fixture.state);
                    let again: Vec<&str> = asked
                        .iter()
                        .map(|ask| ask.id.as_str())
                        .filter(|id| answered.contains(id))
                        .collect();
                    ensure(again.is_empty(), || format!("asked again: {again:?}"))?;
                    let before = follow::before(&answers, &fixture.state);
                    ensure(
                        before.len() == 2 && before.iter().all(|ask| ask.before),
                        || format!("folded: {}", before.len()),
                    )?;
                    let settled = follow::settled(&fixture.settings, &fixture.state);
                    ensure(
                        answered
                            .iter()
                            .all(|id| settled.iter().any(|key| key == id)),
                        || format!("a model may ask: {settled:?}"),
                    )
                }),
                app(
                    "the opener is short: one line of updates, then bullets",
                    |fixture| {
                        let since = fixtures::since(fixture, &[finished(fixture)]);
                        let opener = since::paragraph(&since);
                        let lines: Vec<&str> = opener
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .collect();
                        let bullets = lines.iter().filter(|line| line.starts_with("- ")).count();
                        let updates = lines
                            .iter()
                            .filter(|line| line.contains(&since::date(earlier())))
                            .count();
                        ensure(
                            (1..=5).contains(&bullets)
                                && updates == 1
                                && lines.len() <= bullets + 2,
                            || {
                                format!(
                                    "{bullets} bullets, {updates} update lines, {} lines:\n{opener}",
                                    lines.len()
                                )
                            },
                        )?;
                        ensure(opener.chars().count() <= 700, || {
                            format!("{} characters", opener.chars().count())
                        })
                    },
                ),
                app("putting ё back isn't a fix worth telling", |fixture| {
                    // It is the fixture's most frequent change.
                    let used = fixtures::used(fixture).ok_or("the use wasn't read")?;
                    let spelling: Vec<String> = used
                        .fixes
                        .iter()
                        .filter(|fix| fix.written.contains('ё'))
                        .map(|fix| format!("{} → {}", fix.heard, fix.written))
                        .collect();
                    ensure(spelling.is_empty(), || format!("{spelling:?}"))?;
                    ensure(
                        used.fixes.first().is_some_and(|fix| fix.written == "Codex"),
                        || format!("{:?}", used.fixes),
                    )
                }),
                app(
                    "the use shows real terms, with how they were heard",
                    |fixture| {
                        let terms = candidates(fixture);
                        let codex = terms.iter().find(|term| term.term == "Codex");
                        ensure(
                            codex.is_some_and(|term| {
                                term.sure && term.sounds_like.iter().any(|heard| heard == "кодекс")
                            }),
                            || format!("{terms:?}"),
                        )?;
                        let unsure = terms.iter().filter(|term| !term.sure).count();
                        ensure(unsure > 0, || {
                            "every term is sure: nothing tells checked from unchecked".into()
                        })
                    },
                ),
                app("no ordinary English word is a term", |fixture| {
                    let ordinary: Vec<String> = candidates(fixture)
                        .into_iter()
                        .map(|candidate| candidate.term)
                        .filter(|term| ORDINARY.contains(&term.to_lowercase().as_str()))
                        .collect();
                    ensure(ordinary.is_empty(), || format!("{ordinary:?}"))
                }),
                app("no term Vocabulary already has is offered", |fixture| {
                    let known: Vec<String> = candidates(fixture)
                        .into_iter()
                        .map(|candidate| candidate.term)
                        .filter(|term| {
                            fixture
                                .settings
                                .vocabulary
                                .iter()
                                .any(|entry| entry.term.eq_ignore_ascii_case(term))
                        })
                        .collect();
                    ensure(known.is_empty(), || format!("{known:?}"))
                }),
                app(
                    "the app's plan lists a term a row, only the sure ones checked",
                    |fixture| {
                        let terms = candidates(fixture);
                        let card = plan::with_terms(
                            Vec::new(),
                            &fixtures::answers(),
                            &terms,
                            &fixture.settings,
                            &fixture.state,
                        )
                        .ok_or("no plan")?;
                        let items: Vec<&Item> = card.items.iter().collect();
                        let rows = term_rows(&items, &terms)?;
                        ensure(rows >= 2, || format!("{rows} term rows"))
                    },
                ),
            ],
            steps: vec![
                stage(
                    look_stage,
                    &[LOOK],
                    vec![
                        sent_has(
                            "the 2 to 4 suggestions rule, not a fixed three",
                            &["2 to 4 suggestions"],
                        ),
                        sent_has("what the use shows", &["<since_last_setup>", "<shown>"]),
                        check(
                            "suggestions only: no card, no Vocabulary before the answers",
                            |outcome| {
                                ensure(outcome.turn.cards.is_empty(), || {
                                    format!("{:?}", outcome.turn.cards)
                                })
                            },
                        ),
                        check("two to four suggestions, each kept", |outcome| {
                            let items = list_items(&outcome.turn.answer);
                            ensure((2..=4).contains(&items), || {
                                format!("{items} items: {}", outcome.turn.answer)
                            })
                        }),
                        offline("all four played suggestions stay", |outcome| {
                            ensure(list_items(&outcome.turn.answer) == 4, || {
                                outcome.turn.answer.clone()
                            })
                        }),
                        check(
                            "round one comes after, folded, as answered last time",
                            |outcome| {
                                let turn = &outcome.turn;
                                let chosen = |id: &str| {
                                    turn.asks
                                        .iter()
                                        .find(|ask| ask.id == id)
                                        .map(|ask| ask.chosen.clone())
                                        .unwrap_or_default()
                                };
                                ensure(
                                    turn.asks.len() == 4
                                        && turn.folded
                                        && chosen("cleanup") == ["codex"],
                                    || {
                                        format!(
                                            "{} asks, folded {}, cleanup {:?}",
                                            turn.asks.len(),
                                            turn.folded,
                                            chosen("cleanup")
                                        )
                                    },
                                )
                            },
                        ),
                        answer_in_russian(),
                    ],
                ),
                stage(
                    follow_stage,
                    &["Ещё вопрос.\n\n```hex-ask\n\
                       {\"id\":\"labs.smart_spacing\",\"q\":\"Ставить пробел после вставленного слова?\",\"options\":[{\"id\":\"yes\",\"label\":\"Да, включить\"},{\"id\":\"no\",\"label\":\"Нет\"}]}\n\
                       ```\n\nSources: none"],
                    vec![check("nothing answered before is asked again", |outcome| {
                        let answered: Vec<&str> = outcome
                            .fixture
                            .state
                            .answered
                            .iter()
                            .map(|ask| ask.id.as_str())
                            .collect();
                        let again: Vec<&str> = outcome
                            .turn
                            .asks
                            .iter()
                            .chain(&outcome.turn.more)
                            .filter(|ask| !ask.before && answered.contains(&ask.id.as_str()))
                            .map(|ask| ask.id.as_str())
                            .collect();
                        ensure(again.is_empty(), || format!("asked again: {again:?}"))
                    })],
                ),
                stage(
                    plan_stage,
                    &[PLAN_TERMS],
                    vec![
                        check("a term a row, only the sure ones checked", |outcome| {
                            let items = plan_items(&outcome.turn.cards);
                            term_rows(&items, &candidates(outcome.fixture)).map(|_| ())
                        }),
                        check(
                            "no ordinary English word among the plan's terms",
                            |outcome| {
                                let ordinary: Vec<String> = plan_items(&outcome.turn.cards)
                                    .iter()
                                    .filter(|item| item.key == vocab::KEY)
                                    .flat_map(|item| vocab::terms(&item.to))
                                    .map(|term| term.term)
                                    .filter(|term| ORDINARY.contains(&term.to_lowercase().as_str()))
                                    .collect();
                                ensure(ordinary.is_empty(), || format!("{ordinary:?}"))
                            },
                        ),
                        offline("the use's terms are in the plan", |outcome| {
                            let items = plan_items(&outcome.turn.cards);
                            let rows = items.iter().filter(|item| item.key == vocab::KEY).count();
                            ensure(rows >= 2, || format!("{rows} term rows"))
                        }),
                    ],
                ),
            ],
        },
        Scenario {
            id: "12-your-own-values",
            about: "A plan came with the person's own choices checked for change.",
            fixture: own_model,
            app: vec![app(
                "a line over the person's own choice starts unchecked",
                |fixture| {
                    // They chose GigaAM v3 themselves; the rules would suggest
                    // another model for code in Russian.
                    let card = plan::with_terms(
                        Vec::new(),
                        &fixtures::answers(),
                        &[],
                        &fixture.settings,
                        &fixture.state,
                    )
                    .ok_or("no plan")?;
                    let yours: Vec<&Item> = card.items.iter().filter(|item| item.yours).collect();
                    ensure(!yours.is_empty(), || {
                        format!(
                            "no line over their own value: {:?}",
                            card.items.iter().map(|item| &item.key).collect::<Vec<_>>()
                        )
                    })?;
                    ensure(yours.iter().all(|item| !item.included), || {
                        format!(
                            "checked: {:?}",
                            yours
                                .iter()
                                .filter(|item| item.included)
                                .map(|item| &item.key)
                                .collect::<Vec<_>>()
                        )
                    })
                },
            )],
            steps: vec![stage(
                plan_stage,
                &["Вот что подходит.\n\n```hex-plan\n\
                   {\"key\":\"labs.failure_notices\",\"value\":true,\"why\":\"видно, когда не вышло\"}\n\
                   {\"key\":\"labs.spoken_symbols\",\"value\":true,\"why\":\"знаки голосом\"}\n\
                   {\"key\":\"labs.filler_words\",\"value\":true,\"why\":\"без «эээ»\"}\n\
                   {\"key\":\"labs.history_stats\",\"value\":true,\"why\":\"сколько надиктовано\"}\n\
                   {\"key\":\"labs.vocabulary_biasing\",\"value\":true,\"why\":\"термины из словаря\"}\n\
                   ```\n\nSources: none"],
                vec![
                    check("their own values come unchecked", |outcome| {
                        let checked: Vec<&str> = plan_items(&outcome.turn.cards)
                            .iter()
                            .filter(|item| item.yours && item.included)
                            .map(|item| item.key.as_str())
                            .collect();
                        ensure(checked.is_empty(), || format!("checked: {checked:?}"))
                    }),
                    offline("every line that fits stays: no cap of three", |outcome| {
                        let model: Vec<&str> = plan_items(&outcome.turn.cards)
                            .iter()
                            .map(|item| item.key.as_str())
                            .filter(|key| {
                                [
                                    "labs.failure_notices",
                                    "labs.spoken_symbols",
                                    "labs.filler_words",
                                    "labs.history_stats",
                                    "labs.vocabulary_biasing",
                                ]
                                .contains(key)
                            })
                            .collect();
                        ensure(model.len() > 3, || format!("kept {model:?}"))
                    }),
                ],
            )],
        },
        Scenario {
            id: "13-language",
            about: "With Russian speech and an English window, setup's questions and chips came in \
                    English; an English message was answered in Russian.",
            fixture: newcomer,
            app: vec![
                app(
                    "the app's own questions and options read in Russian",
                    |fixture| {
                        let mut asks = setup::questions(fixture.state.os);
                        asks.extend(follow::candidates(
                            &fixtures::answers(),
                            &fixture.settings,
                            &fixture.state,
                        ));
                        asks_in_russian(&asks)
                    },
                ),
                app("its controls read in Russian", |_| {
                    for words in ["Continue", "Skip", "Not now", follow::YES, follow::NO] {
                        ensure(russian(tr(words)), || format!("{words}: {}", tr(words)))?;
                    }
                    Ok(())
                }),
            ],
            steps: vec![
                message(
                    "Помоги настроить VibeHex",
                    &[ROUND_ONE],
                    vec![
                        setup_route(),
                        check(
                            "the questions and their options are in Russian",
                            |outcome| {
                                ensure(!outcome.turn.asks.is_empty(), || "no questions".into())?;
                                let asks: Vec<Ask> = outcome.turn.asks.clone();
                                crate::assistant::language::within(
                                    &outcome.fixture.settings,
                                    || asks_in_russian(&asks),
                                )
                            },
                        ),
                    ],
                ),
                message(
                    "How do I turn on meeting recording?",
                    &[
                        "Turn on **Meetings**: VibeHex then offers to record when a call starts.\n\n\
                       ```hex-change\n{\"key\":\"meetings.enabled\",\"value\":true,\"why\":\"records calls\"}\n```\n\n\
                       Sources: labs/meetings",
                    ],
                    vec![
                        // The rule stands by the question, after the pages
                        // and the earlier turns in Russian.
                        sent_has(
                            "the language rule as its last line",
                            &[
                                "</question>\nReply in the language the question above is written in",
                            ],
                        ),
                        answer_in_english(),
                    ],
                ),
                message(
                    "А где потом лежат записи?",
                    &[
                        "Записи лежат в **Meetings**: у каждой встречи своя страница с заметками и \
                       расшифровкой.\n\nSources: topics/meetings",
                    ],
                    vec![answer_in_russian()],
                ),
            ],
        },
        Scenario {
            id: "14-like-denis",
            about: "\"Set me up like Denis\" came with retention checked and the shortcuts mixed \
                    into the first list. It is offered with Russian recognition, as here.",
            fixture: newcomer,
            app: vec![
                app(
                    "the preset is offered with Russian recognition",
                    |fixture| {
                        ensure(preset::offered(&fixture.settings), || "not offered".into())?;
                        // A model that knows only Russian is Russian too.
                        let mut gigaam = fixture.settings.clone();
                        gigaam.transcription.model =
                            crate::transcription::models::TranscriptionModelId::GigaAmV3;
                        gigaam.transcription.language = "auto".into();
                        ensure(preset::offered(&gigaam), || "GigaAM on Auto".into())
                    },
                ),
                app("everything is checked but retention", |fixture| {
                    let planned = preset::plan(&preset::denis(), &fixture.settings, &fixture.state);
                    let card = planned.card.ok_or("no plan")?;
                    ensure(card.items.len() > 5, || {
                        format!("{} rows", card.items.len())
                    })?;
                    for item in &card.items {
                        let retention = item.key.contains("retention");
                        ensure(item.included != retention, || {
                            format!("{}: checked {}", item.key, item.included)
                        })?;
                    }
                    ensure(
                        card.items.iter().any(|item| item.key.contains("retention")),
                        || "no retention row".into(),
                    )
                }),
                app("the shortcuts are a second step of their own", |fixture| {
                    let shortcut = |key: &str| {
                        settings::kinds()
                            .any(|(known, kind, ..)| known == key && kind == "shortcut")
                    };
                    let planned = preset::plan(&preset::denis(), &fixture.settings, &fixture.state);
                    let mixed: Vec<String> = planned
                        .card
                        .iter()
                        .flat_map(|card| &card.items)
                        .map(|item| item.key.clone())
                        .filter(|key| shortcut(key) || key == "shortcut_mode")
                        .collect();
                    ensure(mixed.is_empty(), || format!("in the first list: {mixed:?}"))?;
                    let second = preset::shortcuts(&preset::denis(), &fixture.settings)
                        .ok_or("no shortcuts step")?;
                    ensure(
                        !second.items.is_empty()
                            && second
                                .items
                                .iter()
                                .all(|item| shortcut(&item.key) || item.key == "shortcut_mode"),
                        || {
                            format!(
                                "{:?}",
                                second
                                    .items
                                    .iter()
                                    .map(|item| &item.key)
                                    .collect::<Vec<_>>()
                            )
                        },
                    )
                }),
                app(
                    "no value of the preset is one this install has",
                    |fixture| {
                        let planned =
                            preset::plan(&preset::denis(), &fixture.settings, &fixture.state);
                        let same: Vec<String> = planned
                            .card
                            .iter()
                            .flat_map(|card| &card.items)
                            .filter(|item| {
                                item.key != vocab::KEY
                                    && item.from == item.to
                                    && item.to != Value::Null
                            })
                            .map(|item| item.key.clone())
                            .collect();
                        ensure(same.is_empty(), || format!("{same:?}"))
                    },
                ),
            ],
            steps: vec![message(
                "Настрой как у Дениса",
                &[],
                vec![
                    routed(|route| *route == Route::Preset, "the preset"),
                    check("no model is asked", |outcome| {
                        ensure(outcome.sent.is_none(), || "a prompt was sent".into())
                    }),
                    live("nothing was asked live either", |outcome| {
                        ensure(outcome.turn.answer.is_empty(), || {
                            outcome.turn.answer.clone()
                        })
                    }),
                ],
            )],
        },
        not_offered(
            "14b-like-denis-english",
            "With English recognition the preset, made for Russian dictation, was offered all \
             the same.",
            || fixtures::newcomer_speaking("en"),
            "Set me up like Denis",
        ),
        not_offered(
            "14c-like-denis-auto",
            "With recognition on Auto the preset was offered all the same.",
            || fixtures::newcomer_speaking("auto"),
            "Настрой как у Дениса",
        ),
    ]
}

/// The preset asked for where it isn't offered: no plan, no card, an
/// ordinary question the guide answers honestly.
fn not_offered(
    id: &'static str,
    about: &'static str,
    fixture: fn() -> Fixture,
    asked: &'static str,
) -> Scenario {
    Scenario {
        id,
        about,
        fixture,
        app: vec![
            app("the preset isn't offered", |fixture| {
                ensure(!preset::offered(&fixture.settings), || "offered".into())
            }),
            app(
                "every way of asking for it is an ordinary question",
                |fixture| {
                    for question in [
                        "Set me up like Denis",
                        "set me up like the developer",
                        "Настрой как у Дениса",
                        "сделай настройки как у DenisKey",
                    ] {
                        let route = crate::assistant::intent::route(
                            question,
                            &[],
                            &fixture.settings,
                            fixture.now_ms,
                        );
                        ensure(route == Route::Guide, || format!("{question}: {route:?}"))?;
                    }
                    Ok(())
                },
            ),
        ],
        steps: vec![message(
            asked,
            &[
                "That preset is made for dictating in Russian, so it is offered only when \
                 **General › Language** is Russian. **Set VibeHex up for me** asks a few \
                 questions and fits the setup to how you work.\n\n\
                 ```hex-plan\n\
                 {\"key\":\"labs.russian_vibe_coding\",\"value\":true,\"why\":\"as Denis has it\"}\n\
                 ```\n\nSources: topics/assistant",
            ],
            vec![
                routed(|route| *route == Route::Guide, "the guide"),
                sent_has(
                    "the guide saying whom the preset is for",
                    &["offered only when speech recognition is set to Russian"],
                ),
                // A plan block a model writes here makes nothing.
                check("no preset plan and no card", |outcome| {
                    ensure(
                        outcome.turn.cards.is_empty() && outcome.turn.asks.is_empty(),
                        || format!("{:?} {:?}", outcome.turn.cards, outcome.turn.asks),
                    )
                }),
            ],
        )],
    }
}
