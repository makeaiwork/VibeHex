//! The preset's part of the setup machine: "Set me up like Denis" as one
//! plan, then his shortcuts as a second step of their own: the shortcuts
//! card, saved or put off as any card is.

use super::{DONE, Effect, Env, Flow, SETTINGS, SHORTCUTS};
use crate::assistant::changes::Decision;
use crate::assistant::setup::preset;
use crate::assistant::store::Turn;
use crate::desktop::i18n::{tr, tr_fill, tr_window};

impl Flow {
    /// The owner's settings as one plan, no model asked, and only where
    /// the preset is offered.
    pub(super) fn preset(&mut self, question: String, env: &Env, out: &mut Vec<Effect>) {
        // Made for Russian dictation: with any other recognition there is
        // no way in, and none is made here.
        if env.busy || !preset::offered(env.saved) {
            return;
        }
        let context = self.context(env);
        let planned = preset::plan(&preset::denis(), &context.settings, &context.state);
        let mut answer = if planned.card.is_some() {
            tr("Here is how Denis has VibeHex set up. Untick what you don't need.").to_owned()
        } else {
            tr("Everything already matches how Denis has it.").to_owned()
        };
        if !planned.missing.is_empty() {
            // Places in the window, in the window's words.
            let missing: Vec<&str> = planned.missing.iter().map(|name| tr_window(name)).collect();
            answer.push_str("\n\n");
            answer.push_str(&tr_fill("Not available here: {}", &missing.join(", ")));
        }
        let settled = planned.card.is_none();
        let plan = Turn {
            question,
            answer,
            cards: planned.card.into_iter().collect(),
            preset: SETTINGS.into(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(plan, out);
        if settled {
            self.shortcuts(env, out);
        }
    }

    /// The preset's plan was applied or put off: on to its shortcuts. A
    /// save that failed stays at the plan, to try again.
    pub(super) fn preset_decided(
        &mut self,
        turn: usize,
        card: usize,
        env: &Env,
        out: &mut Vec<Effect>,
    ) {
        let last = self.turns.len().saturating_sub(1);
        let decided = |turn: &Turn| {
            let went = turn.cards.get(card).map(|card| &card.decision);
            turn.preset == SETTINGS
                && matches!(went, Some(Decision::Applied { .. } | Decision::Declined))
        };
        if turn == last && self.turns.get(turn).is_some_and(decided) {
            self.shortcuts(env, out);
        }
    }

    /// The preset's second step: the owner's shortcuts as the shortcuts
    /// card, each row Denis's binding over the person's, recorded anew
    /// where they want another key. "Use these" saves the card, "Keep
    /// mine" puts it off. Already the same everywhere, it says so.
    fn shortcuts(&mut self, env: &Env, out: &mut Vec<Effect>) {
        let Some(card) = preset::shortcuts(&preset::denis(), env.saved) else {
            return;
        };
        let differs = card.items.iter().any(|item| item.included);
        let (answer, cards, step) = match differs {
            true => (
                tr("Here are Denis's shortcuts, beside yours where they differ."),
                vec![card],
                SHORTCUTS,
            ),
            false => (
                tr("Your shortcuts already match Denis's."),
                Vec::new(),
                DONE,
            ),
        };
        let asked = Turn {
            answer: answer.into(),
            cards,
            preset: step.into(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(asked, out);
    }
}
