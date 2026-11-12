//! The assertions scenarios are made of: on the route, on what the model
//! was sent, and on the parsed turn (cards, asks, plan rows, steps,
//! sources, language). None reads an answer's wording beyond a proper name
//! or a keycap the fixture fixes.

use serde_json::Value;

use super::harness::{AppCheck, Check, Fixture, Layer, Outcome, Verdict};
use crate::assistant::changes::{CardKind, Item, names};
use crate::assistant::intent::Route;
use crate::assistant::setup::Ask;

pub fn check(name: impl Into<String>, test: impl Fn(&Outcome) -> Verdict + 'static) -> Check {
    Check {
        name: name.into(),
        layer: Layer::Both,
        test: Box::new(test),
    }
}

/// Only with the played reply.
pub fn offline(name: impl Into<String>, test: impl Fn(&Outcome) -> Verdict + 'static) -> Check {
    Check {
        layer: Layer::Offline,
        ..check(name, test)
    }
}

/// Only on a real engine.
pub fn live(name: impl Into<String>, test: impl Fn(&Outcome) -> Verdict + 'static) -> Check {
    Check {
        layer: Layer::Live,
        ..check(name, test)
    }
}

pub fn app(name: impl Into<String>, test: impl Fn(&Fixture) -> Verdict + 'static) -> AppCheck {
    AppCheck {
        name: name.into(),
        test: Box::new(test),
    }
}

pub fn ensure(holds: bool, why: impl FnOnce() -> String) -> Verdict {
    match holds {
        true => Ok(()),
        false => Err(why()),
    }
}

fn items<'a>(outcome: &'a Outcome) -> impl Iterator<Item = &'a Item> {
    outcome.turn.cards.iter().flat_map(|card| &card.items)
}

fn keys(outcome: &Outcome) -> Vec<String> {
    items(outcome).map(|item| item.key.clone()).collect()
}

/// The checks every step of every scenario is held to.
pub fn always() -> Vec<Check> {
    vec![
        check("answered without an error", |outcome| {
            ensure(outcome.turn.error.is_none(), || {
                format!("{:?}", outcome.turn.error)
            })
        }),
        // Never a card that changes a setting to the value it has: not as
        // stored, and not as the engine it runs (`names::same`).
        check("no card changes a value to itself", |outcome| {
            let settings = &outcome.fixture.settings;
            let same: Vec<&str> = outcome
                .turn
                .cards
                .iter()
                .filter(|card| {
                    !matches!(card.kind, CardKind::Vocabulary | CardKind::VocabularyEdit)
                })
                .flat_map(|card| &card.items)
                .filter(|item| names::same(settings, &item.key, &item.from, &item.to))
                .map(|item| item.key.as_str())
                .collect();
            ensure(same.is_empty(), || format!("from equals to: {same:?}"))
        }),
        // Nothing this computer can't use is offered, in a card or a
        // question.
        check("nothing this computer can't use is offered", |outcome| {
            let hardware = &outcome.fixture.state.hardware;
            let unfit = |key: &str| crate::assistant::setup::follow::unfit(key, hardware);
            let mut offered: Vec<String> =
                keys(outcome).into_iter().filter(|key| unfit(key)).collect();
            offered.extend(
                outcome
                    .turn
                    .asks
                    .iter()
                    .chain(&outcome.turn.more)
                    .filter(|ask| unfit(&ask.id))
                    .map(|ask| ask.id.clone()),
            );
            ensure(offered.is_empty(), || format!("offered: {offered:?}"))
        }),
        no_web_after_personal(),
    ]
}

/// What of the person's own a request's `input` holds, by name: excerpts
/// of their recordings, a list of them, what their use shows, Vocabulary
/// entries; and, on an agent turn, their cleanup instructions (the router
/// sends those with the web on, as it always has). Read off the text the
/// model was sent, not off what the app says it sent.
pub fn holds(input: &str, agent: bool) -> Vec<&'static str> {
    let mut holds = Vec::new();
    let excerpt = |line: &str| {
        line.starts_with('[')
            && ["] Meeting \"", "] File \"", "] Dictation"]
                .iter()
                .any(|kind| line.contains(kind))
    };
    if input.lines().any(excerpt) {
        holds.push("recordings");
    }
    if input.contains("\nid | title | start | length | notes\n")
        || input.contains("\nid | name | added | length\n")
    {
        holds.push("a list of recordings");
    }
    if input.contains("<usage>\n") {
        holds.push("usage");
    }
    let entries = input.split("<vocabulary total=").skip(1).any(|block| {
        let block = block.split("</vocabulary>").next().unwrap_or_default();
        !block.contains("None of the entries is named.")
    });
    let terms = input
        .split("<vocabulary_terms total=\"")
        .skip(1)
        .any(|block| !block.starts_with("0\""));
    if entries || terms {
        holds.push("vocabulary");
    }
    if agent && input.contains("<cleanup_instructions>\nGlobal mode") {
        holds.push("cleanup instructions");
    }
    holds
}

/// The privacy rule every step of every scenario is held to: a request
/// that holds anything of the person's own may not search the web, nor
/// may any request after it in the turn, nor any in a later turn of the
/// conversation, which then never goes online at all.
fn no_web_after_personal() -> Check {
    check("no web once the prompt holds what is theirs", |outcome| {
        if outcome.held_before {
            let web = &outcome.turn.web;
            ensure(!web.went_online() && web.sources.is_empty(), || {
                format!("an earlier turn held what is theirs, and this one went online: {web:?}")
            })?;
        }
        let mut held: Vec<&str> = Vec::new();
        for (index, sent) in outcome.sent.iter().enumerate() {
            if held.is_empty() {
                held = holds(&sent.input, outcome.agent());
            }
            ensure(
                !sent.web || (held.is_empty() && !outcome.held_before),
                || {
                    format!(
                        "request {} may search the web and holds {held:?} (earlier turn: {})",
                        index + 1,
                        outcome.held_before
                    )
                },
            )?;
        }
        Ok(())
    })
}

/// The model called `tool`, on an agent turn (a turn the router read for
/// has no calls to hold).
pub fn called(tool: &'static str) -> Check {
    check(format!("it called {tool}"), move |outcome| {
        ensure(
            !outcome.agent() || outcome.called.iter().any(|called| called == tool),
            || format!("called: {:?}", outcome.called),
        )
    })
}

/// The model called none of `tools`.
pub fn never_called(tools: &'static [&'static str]) -> Check {
    check(
        format!("it never called {}", tools.join(", ")),
        move |outcome| {
            let called: Vec<&String> = outcome
                .called
                .iter()
                .filter(|called| tools.contains(&called.as_str()))
                .collect();
            ensure(called.is_empty(), || format!("called: {called:?}"))
        },
    )
}

/// The turn took at most `most` requests.
pub fn steps_at_most(most: usize) -> Check {
    check(format!("at most {most} requests"), move |outcome| {
        ensure(outcome.sent.len() <= most, || {
            format!("{} requests", outcome.sent.len())
        })
    })
}

/// The turn's trace, as an agent turn keeps it, holds what `want` looks
/// for; a router's turn has none and passes.
pub fn traced(
    name: &'static str,
    want: impl Fn(&[crate::assistant::agent::trace::Did]) -> bool + 'static,
) -> Check {
    check(name, move |outcome| {
        ensure(!outcome.agent() || want(&outcome.turn.trace), || {
            format!("trace: {:?}", outcome.turn.trace)
        })
    })
}

pub fn routed(want: fn(&Route) -> bool, name: &'static str) -> Check {
    check(format!("routed to {name}"), move |outcome| {
        ensure(outcome.route.as_ref().is_some_and(want), || {
            format!("{:?}", outcome.route)
        })
    })
}

pub fn no_cards() -> Check {
    check("no card", |outcome| {
        ensure(outcome.turn.cards.is_empty(), || {
            format!("cards: {:?}", keys(outcome))
        })
    })
}

/// No card changes `key`, or a key under it.
pub fn no_card_for(key: &'static str) -> Check {
    check(format!("no card for {key}"), move |outcome| {
        let found: Vec<String> = keys(outcome)
            .into_iter()
            .filter(|found| found == key || found.starts_with(&format!("{key}.")))
            .collect();
        ensure(found.is_empty(), || format!("cards: {found:?}"))
    })
}

/// No card turns a Labs feature on.
pub fn no_labs_card() -> Check {
    check("no Labs card", |outcome| {
        let found: Vec<String> = keys(outcome)
            .into_iter()
            .filter(|key| key.starts_with("labs."))
            .collect();
        ensure(found.is_empty(), || format!("cards: {found:?}"))
    })
}

/// The gate dropped a proposal whose record line holds `what` and `why`
/// (`offer::record`), with the reply a model is played as writing.
pub fn dropped(what: &'static str, why: &'static str) -> Check {
    offline(format!("the gate dropped {what}"), move |outcome| {
        let lines = crate::assistant::offer::recorded(&outcome.turn.knew);
        ensure(
            lines
                .iter()
                .any(|line| line.contains(what) && line.contains(why)),
            || format!("dropped: {lines:?}"),
        )
    })
}

/// A card changes `key` (to `value`, when given).
pub fn card_for(key: &'static str, value: Option<Value>) -> Check {
    check(format!("a card for {key}"), move |outcome| {
        let hit = items(outcome)
            .any(|item| item.key == key && value.as_ref().is_none_or(|value| item.to == *value));
        ensure(hit, || format!("cards: {:?}", keys(outcome)))
    })
}

/// The answer names every one of `names` (proper names, keycaps).
pub fn answer_names(names: &'static [&'static str]) -> Check {
    check(
        format!("the answer names {}", names.join(", ")),
        move |outcome| {
            let answer = &outcome.turn.answer;
            let missing: Vec<&str> = names
                .iter()
                .copied()
                .filter(|name| !answer.contains(name))
                .collect();
            ensure(missing.is_empty(), || {
                format!("missing {missing:?} in: {answer}")
            })
        },
    )
}

/// Where `name` first stands in the answer, ignoring case.
fn at(answer: &str, name: &str) -> Option<usize> {
    answer.to_lowercase().find(&name.to_lowercase())
}

/// One of `first` stands in the answer before any of `later` does.
pub fn names_before(first: &'static [&'static str], later: &'static [&'static str]) -> Check {
    check(
        format!("{} before {}", first.join(" or "), later.join(" or ")),
        move |outcome| {
            let answer = &outcome.turn.answer;
            let place = |names: &[&str]| names.iter().filter_map(|name| at(answer, name)).min();
            let holds = match (place(first), place(later)) {
                (Some(first), Some(later)) => first < later,
                (Some(_), None) => true,
                (None, _) => false,
            };
            ensure(holds, || format!("in: {answer}"))
        },
    )
}

/// The share of Cyrillic among an answer's letters, in percent.
fn cyrillic(text: &str) -> usize {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    let cyrillic = letters
        .iter()
        .filter(|c| ('\u{0400}'..='\u{04FF}').contains(*c))
        .count();
    cyrillic * 100 / letters.len().max(1)
}

/// Written in Russian: window names stay English, so a third is enough.
pub fn russian(text: &str) -> bool {
    cyrillic(text) >= 30
}

pub fn answer_in_russian() -> Check {
    check("the answer is in Russian", |outcome| {
        ensure(russian(&outcome.turn.answer), || {
            outcome.turn.answer.clone()
        })
    })
}

pub fn answer_in_english() -> Check {
    check("the answer is in English", |outcome| {
        ensure(cyrillic(&outcome.turn.answer) <= 5, || {
            outcome.turn.answer.clone()
        })
    })
}

/// Every question asked, and each of its options, reads in Russian: the
/// model's as written, the app's own as the window shows them.
pub fn asks_in_russian(asks: &[Ask]) -> Verdict {
    for ask in asks {
        let question = match ask.own {
            true => crate::desktop::i18n::tr(&ask.q).to_owned(),
            false => ask.q.clone(),
        };
        ensure(russian(&question), || format!("question: {question}"))?;
        for option in &ask.options {
            let label = match ask.own {
                true => crate::desktop::i18n::tr(&option.label).to_owned(),
                false => option.label.clone(),
            };
            // A name (Codex, Parakeet v3) reads the same in any language.
            let named = label.chars().filter(|c| c.is_alphabetic()).count() > 0
                && cyrillic(&label) == 0
                && label.split_whitespace().count() <= 3;
            ensure(russian(&label) || named, || {
                format!("option of {}: {label}", ask.id)
            })?;
        }
    }
    Ok(())
}

/// What the model was sent holds every one of `parts`.
pub fn sent_has(what: &'static str, parts: &'static [&'static str]) -> Check {
    offline(format!("the prompt has {what}"), move |outcome| {
        if outcome.sent.is_empty() {
            return Err("no model was asked".into());
        }
        let all = outcome.read();
        let missing: Vec<&str> = parts
            .iter()
            .copied()
            .filter(|part| !all.contains(part))
            .collect();
        ensure(missing.is_empty(), || format!("missing {missing:?}"))
    })
}

/// What the model was sent holds none of `parts`.
pub fn sent_lacks(what: &'static str, parts: &'static [&'static str]) -> Check {
    offline(format!("the prompt has no {what}"), move |outcome| {
        let all = outcome.read();
        let found: Vec<&str> = parts
            .iter()
            .copied()
            .filter(|part| all.contains(part))
            .collect();
        ensure(found.is_empty(), || format!("found {found:?}"))
    })
}

/// The answer stayed off the web: it wasn't allowed to search, and shows
/// no web source.
pub fn web_off() -> Check {
    check("the web stayed off", |outcome| {
        let allowed = outcome.sent.iter().any(|sent| sent.web);
        let web = &outcome.turn.web;
        ensure(
            !allowed && !web.went_online() && web.sources.is_empty(),
            || format!("allowed {allowed}, web {web:?}"),
        )
    })
}

/// The `[n]` marks an answer carries, each once.
pub fn marks(answer: &str) -> Vec<u32> {
    let mut marks = Vec::new();
    let mut rest = answer;
    while let Some(open) = rest.find('[') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find(']') else {
            break;
        };
        // `[2]`, `[2.3]`, `[2.3-2.5]`, `[2.3, 4.1]`: the excerpt is the
        // number before the dot.
        for part in rest[..close].split([',', '-']) {
            let excerpt = part.trim().split('.').next().unwrap_or_default();
            if let Ok(n) = excerpt.parse::<u32>()
                && !marks.contains(&n)
            {
                marks.push(n);
            }
        }
    }
    marks
}
