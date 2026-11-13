//! The scenario about round one's own words (`setup::about`): written
//! under the chips, they reach the model in a tag of their own and steer
//! its follow-ups, never quoted back.

use super::checks::{app, check, ensure, sent_has};
use super::fixtures::newcomer;
use super::harness::Scenario;
use super::setups::stage;
use super::stages::{FOLLOW_CODE, IN_THEIR_WORDS, follow_stage, words_follow_stage};
use crate::assistant::setup::Ask;

/// Words a question about code names, in English or Russian.
const CODE: [&str; 10] = [
    "code",
    "код",
    "terminal",
    "терминал",
    "editor",
    "редактор",
    "symbol",
    "символ",
    "программ",
    "program",
];

/// Whether a follow-up is about code: a feature meant only for it, or its
/// words name it.
fn about_code(ask: &Ask) -> bool {
    const ONLY: [&str; 4] = [
        "labs.screenshots",
        "labs.russian_vibe_coding",
        "labs.terminal_paste_shortcut",
        "vocabulary.code_terms",
    ];
    let q = ask.q.to_lowercase();
    ONLY.contains(&ask.id.as_str()) || CODE.iter().any(|word| q.contains(word))
}

pub fn scenario() -> Scenario {
    Scenario {
        id: "11b-in-their-words",
        about: "Round one had nowhere to say anything in one's own words. Written under it \
                (\"I write code in Russian and dictate into Cursor\") with only dictation \
                into any app picked, the words reach the model in their own tag and steer \
                the follow-ups toward code, never quoted back.",
        fixture: newcomer,
        app: vec![app("empty, no tag is sent", |fixture| {
            let (_, stage) = follow_stage(fixture);
            ensure(stage.about().is_empty(), || stage.about().to_owned())
        })],
        steps: vec![stage(
            words_follow_stage,
            &[FOLLOW_CODE],
            vec![
                sent_has(
                    "their own words in a tag of their own",
                    &[
                        "<in_their_words>\nI write code in Russian and dictate into Cursor\n</in_their_words>",
                        "Never quote it back",
                    ],
                ),
                check("follow-ups were asked", |outcome| {
                    ensure(!outcome.turn.asks.is_empty(), || "none".into())
                }),
                check("a follow-up is about code", |outcome| {
                    ensure(outcome.turn.asks.iter().any(about_code), || {
                        format!(
                            "{:?}",
                            outcome
                                .turn
                                .asks
                                .iter()
                                .map(|ask| (&ask.id, &ask.q))
                                .collect::<Vec<_>>()
                        )
                    })
                }),
                check("their words aren't quoted back", |outcome| {
                    let mut written = outcome.turn.answer.to_lowercase();
                    for ask in &outcome.turn.asks {
                        written.push('\n');
                        written.push_str(&ask.q.to_lowercase());
                    }
                    let words: Vec<&str> = IN_THEIR_WORDS.split_whitespace().collect();
                    let quoted: Vec<String> = words
                        .windows(4)
                        .map(|four| four.join(" ").to_lowercase())
                        .filter(|four| written.contains(four.as_str()))
                        .chain(
                            ["пишу код на русском", "диктую в cursor"]
                                .into_iter()
                                .filter(|phrase| written.contains(phrase))
                                .map(String::from),
                        )
                        .collect();
                    ensure(quoted.is_empty(), || format!("quoted: {quoted:?}"))
                }),
            ],
        )],
    }
}
