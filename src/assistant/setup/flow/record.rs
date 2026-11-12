//! The setup machine's record: what a setup keeps of its answers and of
//! how its plan went, following the plan card's decision; and what stands
//! when a model could not be asked.

use super::{Effect, Env, Flow};
use crate::assistant::changes::{Card, CardKind, Decision};
use crate::assistant::setup::{Stage, past};
use crate::assistant::store::Turn;
use crate::desktop::i18n::tr;

/// What a round or a plan that could not be asked for says, and the open
/// round beside its Continue while it waits.
pub const BUSY: &str = "The model is answering in another conversation.";

impl Flow {
    /// A setup's plan card was decided, or decided again: applied or put
    /// off, the setup is finished, and kept with what it applied and what
    /// the person declined; undone, its record no longer says applied. A
    /// save that failed decides nothing: the record waits for the one
    /// that goes through.
    pub(super) fn plan_decided(
        &mut self,
        turn: usize,
        card: usize,
        env: &Env,
        out: &mut Vec<Effect>,
    ) {
        let Some(plan) = self.turns.get(turn) else {
            return;
        };
        let Some(card) = plan.cards.get(card) else {
            return;
        };
        if plan.answers.is_empty() || plan.follow || card.kind != CardKind::Plan {
            return;
        }
        if matches!(card.decision, Decision::Proposed | Decision::Failed { .. }) {
            return;
        }
        let plan_ms = plan.created_ms;
        // An older record, kept once decided, has no conversation to
        // update it by.
        let older = |setup: &past::Setup| setup.conversation == 0 && setup.at_ms == plan_ms;
        if self.kept.iter().any(older) {
            return;
        }
        let card = card.clone();
        self.keep(Some((&card, plan_ms)), env, out);
    }

    /// Keeps the setup with every answer so far, and with how its plan
    /// went once decided (`plan`: the card and its turn's time); a later
    /// call updates the same record. The one place a setup's record is
    /// made.
    pub(super) fn keep(&mut self, plan: Option<(&Card, u64)>, env: &Env, out: &mut Vec<Effect>) {
        let asked = self.asked();
        let answers = crate::assistant::setup::answers(&asked);
        let started_ms = self
            .setup_turns()
            .first()
            .map(|turn| turn.created_ms)
            .unwrap_or_default();
        let at_ms = plan.map_or(env.now_ms, |(_, at)| at);
        let mut setup = past::finished(
            &asked,
            &answers,
            plan.map(|(card, _)| card),
            env.saved,
            at_ms,
        );
        setup.conversation = self.conversation;
        setup.started_ms = started_ms;
        setup.plan_ms = plan.map_or(0, |(_, at)| at);
        self.emit(Effect::Keep(setup), out);
    }

    /// A model could not be asked after all. A returning setup's look:
    /// round one at once, as on a built-in model. A round's follow-ups or
    /// its plan: a turn that says why, to try again from.
    pub(super) fn unasked(&mut self, stage: Stage, env: &Env, out: &mut Vec<Effect>) {
        let turn = match stage {
            Stage::Look {
                facts, asks, terms, ..
            } => Turn {
                asks,
                facts,
                folded: true,
                terms,
                ..Turn::default()
            },
            Stage::Follow { answers, line, .. } => Turn {
                question: line,
                answers,
                follow: true,
                error: Some(tr(BUSY).into()),
                ..Turn::default()
            },
            Stage::Plan { answers, line, .. } => Turn {
                question: line,
                answers,
                error: Some(tr(BUSY).into()),
                ..Turn::default()
            },
            Stage::None | Stage::Ask => return,
        };
        let turn = Turn {
            created_ms: env.now_ms,
            ..turn
        };
        self.push(turn, out);
    }
}
