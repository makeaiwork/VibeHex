//! A model's reply, read: its change lines through the gate
//! (`assistant::offer`), as an answer's or a plan's, into the cards, the
//! refusals, setup's questions, and what was dropped and why.
//!
//! A remote model is asked once more, silently: to fix lines the app
//! refused, or to write its whole reply again when it offered a value
//! that is set already or a setting this computer can't have, so its
//! words claim no card there isn't. A built-in model isn't asked again.

use std::sync::atomic::{AtomicBool, Ordering};

use super::changes::parse::Parsed;
use super::changes::{self, Card, CardKind, Refused, Resolved, Why};
use super::offer::{Dropped, Gate, Origin};
use super::prompt;
use super::setup::{self, Ask, Stage};
use super::worker::{Generate, Question};

/// A reply with its change lines read and gated.
#[derive(Default)]
pub(super) struct Settled {
    pub reply: String,
    pub cards: Vec<Card>,
    pub refused: Vec<Refused>,
    /// Setup's questions, when it asked them.
    pub asks: Vec<Ask>,
    /// What the gate dropped, for the turn's record.
    pub dropped: Vec<Dropped>,
}

/// Reads `reply` against the question's settings. `again` is there for a
/// remote model, and is called before it is asked once more.
pub(super) fn settle(
    question: &Question,
    prompt: &prompt::Prompt,
    reply: String,
    generate: &Generate,
    cancel: &AtomicBool,
    again: Option<fn()>,
) -> Settled {
    let mut parsed = changes::parse::parse(&reply);
    let plan = matches!(question.setup, Stage::Plan { .. });
    let os = question
        .changes
        .as_ref()
        .map_or(std::env::consts::OS, |context| context.state.os);
    let asks = match (&question.setup, &question.changes) {
        (Stage::Ask, context) => {
            let mut asks = setup::asks(&parsed.asks, os);
            if let Some(context) = context {
                setup::preselect(&mut asks, &context.settings, &context.state);
            }
            asks
        }
        // Follow-up questions only: any change lines are dropped.
        (Stage::Follow { candidates, .. }, Some(context)) => {
            let gate = Gate::new(&context.settings, &context.state);
            let (asks, dropped) = setup::follow::follow_ups(&parsed.asks, candidates, &gate);
            return Settled {
                reply,
                asks,
                dropped,
                ..Settled::default()
            };
        }
        (Stage::Follow { candidates, .. }, None) => candidates.clone(),
        // Suggestions only: round one comes after them, as it was.
        (Stage::Look { asks, .. }, _) => {
            return Settled {
                reply,
                asks: asks.clone(),
                ..Settled::default()
            };
        }
        _ => Vec::new(),
    };
    let Some(context) = &question.changes else {
        return Settled {
            reply,
            asks,
            ..Settled::default()
        };
    };
    // Setup's answer is one list, whichever fence each line came in.
    if plan {
        changes::parse::as_plan(&mut parsed.proposals);
    }
    let origin = match plan {
        true => Origin::Plan,
        false => Origin::Answer,
    };
    let settings = &context.settings;
    let gate = Gate::new(settings, &context.state).asked(&question.question);
    let read = |parsed: &Parsed| {
        let mut read = changes::resolve(&parsed.proposals, &gate, origin);
        read.refused.extend(Refused::malformed(&parsed.malformed));
        read
    };
    let Resolved {
        mut cards,
        mut refused,
        mut dropped,
    } = read(&parsed);
    // An answer's words may claim a difference there isn't, or a card
    // there can't be; a plan just leaves such lines out.
    let (already, not_here) = match plan {
        true => (Vec::new(), Vec::new()),
        false => {
            let first = Resolved {
                dropped: dropped.clone(),
                ..Resolved::default()
            };
            (first.already(settings), first.not_here(settings))
        }
    };
    let mut reply = reply;
    let fixable: Vec<Refused> = refused
        .iter()
        .filter(|refused| refused.why != Why::Never)
        .cloned()
        .collect();
    let rewrite = !already.is_empty() || !not_here.is_empty();
    let asking = (rewrite || !fixable.is_empty()) && !cancel.load(Ordering::Acquire);
    if let Some(again) = again.filter(|_| asking) {
        again();
    }
    if again.is_some() && asking && rewrite {
        // The whole reply again, told what is set already and what this
        // computer can't have; its own refusals fixed with it.
        let again = prompt::rewrite(prompt, &reply, &already, &not_here, &fixable);
        if let Ok(fixed) = generate(&again.instructions, &again.input, cancel, None)
            && !changes::parse::parse(&fixed).body.trim().is_empty()
        {
            let more = read(&changes::parse::parse(&fixed));
            cards = more.cards;
            refused = more.refused;
            for drop in more.dropped {
                if !dropped.contains(&drop) {
                    dropped.push(drop);
                }
            }
            reply = fixed;
        }
    } else if again.is_some() && asking {
        let again = prompt::repair(prompt, &reply, &fixable);
        if let Ok(fixed) = generate(&again.instructions, &again.input, cancel, None) {
            let mut parsed = changes::parse::parse(&fixed);
            if plan {
                changes::parse::as_plan(&mut parsed.proposals);
            }
            let more = changes::resolve(&parsed.proposals, &gate, origin);
            let fixed: Vec<String> = more
                .cards
                .iter()
                .flat_map(|card| card.items.iter().map(|item| item.key.clone()))
                .collect();
            for card in more.cards {
                // Fixed terms join the terms card there is.
                if card.kind == CardKind::Vocabulary
                    && let Some(terms) = cards
                        .iter_mut()
                        .find(|known| known.kind == CardKind::Vocabulary)
                {
                    changes::vocab::merge(terms, card);
                } else if card.kind == CardKind::VocabularyEdit
                    && let Some(edits) = cards
                        .iter_mut()
                        .find(|known| known.kind == CardKind::VocabularyEdit)
                {
                    changes::vocab_edit::merge(edits, card);
                } else if !cards.iter().any(|known| known.same_keys(&card)) {
                    cards.push(card);
                }
            }
            // A refusal its fix replaced goes; the rest stay.
            if !fixed.is_empty() {
                refused.retain(|refused| {
                    refused.why == Why::Never
                        || refused.key.as_ref().is_some_and(|key| !fixed.contains(key))
                });
            }
        }
    }
    if plan {
        // The app's rules decide; the model's lines join them.
        let terms = match &question.setup {
            Stage::Plan { terms, .. } => terms.as_slice(),
            _ => &[],
        };
        let (card, more) = setup::plan::planned(
            cards,
            question.setup.answers(),
            terms,
            settings,
            &context.state,
        );
        cards = card.into_iter().collect();
        dropped.extend(more);
    }
    // A question about one's shortcuts gets the shortcuts card, whatever
    // the model wrote: the keys it named for them join it.
    if question.setup == Stage::None
        && let Some(only) = super::intent::shortcuts(&question.question)
        && let Some(mut card) =
            changes::shortcuts::card(settings, &Default::default(), &Default::default(), &only)
    {
        changes::shortcuts::take(&mut card, &mut cards);
        cards.push(card);
    }
    Settled {
        reply,
        cards,
        refused,
        asks,
        dropped,
    }
}
