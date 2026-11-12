//! The scenarios about the gate (`assistant::offer`): what a model
//! proposes and the app doesn't offer, with why in the turn's record.

use super::answers::message;
use super::checks::{app, check, dropped, ensure, no_card_for, offline, routed, sent_has};
use super::fixtures::owner;
use super::harness::{Fixture, Scenario};
use crate::assistant::changes::{CardKind, vocab};
use crate::assistant::intent::Route;
use crate::assistant::setup::preset;

/// A Labs switch another OS has and this one doesn't, the feature's name,
/// and the OS that has it.
#[cfg(target_os = "macos")]
const ELSEWHERE: (&str, &str, &str) = ("labs.skip_pauses", "Skip long pauses", "Linux");
#[cfg(not(target_os = "macos"))]
const ELSEWHERE: (&str, &str, &str) = ("labs.smart_spacing", "Smart spacing", "macOS");

#[cfg(target_os = "macos")]
const TURN_ON: &str = "Turn on Skip long pauses";
#[cfg(not(target_os = "macos"))]
const TURN_ON: &str = "Turn on Smart spacing";

/// The reply a model is played as writing, with a card for it, and the one
/// it writes again once told this computer can't have it.
#[cfg(target_os = "macos")]
const OFFERED: [&str; 2] = [
    "Turn on **Labs › Skip long pauses**.\n\n```hex-change\n\
     {\"key\":\"labs.skip_pauses\",\"value\":true,\"why\":\"leaves the silence out\"}\n\
     ```\n\nSources: labs/skip-pauses",
    "**Skip long pauses** is in VibeHex on Linux and Windows: this Mac doesn't have it.\n\n\
     Sources: labs/skip-pauses",
];
#[cfg(not(target_os = "macos"))]
const OFFERED: [&str; 2] = [
    "Turn on **Labs › Smart spacing**.\n\n```hex-change\n\
     {\"key\":\"labs.smart_spacing\",\"value\":true,\"why\":\"spaces around what is pasted\"}\n\
     ```\n\nSources: labs/smart-spacing",
    "**Smart spacing** is a macOS feature: this computer doesn't have it.\n\n\
     Sources: labs/smart-spacing",
];

/// Ordinary words a model is played as offering for Vocabulary beside a
/// name: a capital, Title Case, and a word heard right in Russian letters.
const ORDINARY: [&str; 3] = ["web", "new session", "monkey"];

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "19-not-on-this-computer",
            about: "A card for a Labs feature this OS doesn't have came blocked from an answer \
                    and was left out of a setup: now neither offers it, and the answer says so.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                TURN_ON,
                &OFFERED,
                vec![
                    routed(|route| *route == Route::Guide, "the guide"),
                    no_card_for(ELSEWHERE.0),
                    dropped(ELSEWHERE.0, "not available here"),
                    offline("the reply written again is the answer", |outcome| {
                        let answer = &outcome.turn.answer;
                        ensure(
                            answer.contains(ELSEWHERE.1) && answer.contains(ELSEWHERE.2),
                            || answer.clone(),
                        )
                    }),
                ],
            )],
        },
        Scenario {
            id: "20-ordinary-words-are-no-terms",
            about: "Vocabulary was offered ordinary English words on their capital alone: Web, \
                    New Session, Monkey.",
            fixture: owner,
            app: Vec::new(),
            steps: vec![message(
                "Какие слова мне стоит добавить в словарь?",
                &["Вот что стоит добавить.\n\n```hex-vocab\n\
                   {\"term\":\"Supabase\",\"sounds_like\":[\"супабейс\"]}\n\
                   {\"term\":\"Web\",\"sounds_like\":[\"веб\"]}\n\
                   {\"term\":\"New Session\"}\n\
                   {\"term\":\"Monkey\",\"sounds_like\":[\"манки\"]}\n\
                   ```\n\nSources: topics/vocabulary"],
                vec![
                    check("no ordinary word is offered as a term", |outcome| {
                        let offered: Vec<String> = outcome
                            .turn
                            .cards
                            .iter()
                            .filter(|card| card.kind == CardKind::Vocabulary)
                            .flat_map(|card| &card.items)
                            .flat_map(|item| vocab::terms(&item.to))
                            .map(|term| term.term)
                            .filter(|term| ORDINARY.contains(&term.to_lowercase().as_str()))
                            .collect();
                        ensure(offered.is_empty(), || format!("{offered:?}"))
                    }),
                    offline("the name is offered", |outcome| {
                        let names: Vec<String> = outcome
                            .turn
                            .cards
                            .iter()
                            .flat_map(|card| &card.items)
                            .flat_map(|item| vocab::terms(&item.to))
                            .map(|term| term.term)
                            .collect();
                        ensure(names == ["Supabase"], || format!("{names:?}"))
                    }),
                    dropped("term Web", "an ordinary word heard right"),
                    dropped("term New Session", "an ordinary word heard right"),
                    dropped("term Monkey", "an ordinary word heard right"),
                ],
            )],
        },
    ]
}

/// The preset asked for where it isn't offered: no plan, no card, an
/// ordinary question the guide answers honestly.
pub fn not_offered(
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
                dropped("labs.russian_vibe_coding", "the preset isn't offered"),
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
