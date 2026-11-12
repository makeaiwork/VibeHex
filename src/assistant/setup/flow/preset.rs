//! The preset's part of the setup machine: "Set me up like Denis" as one
//! plan, then his shortcuts as a second step of their own.

use super::{DONE, Effect, Env, Flow, SETTINGS, SHORTCUTS, SHORTCUTS_PLAN};
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
            let missing: Vec<&str> = planned.missing.iter().map(|name| tr(name)).collect();
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

    /// The preset's second step: the owner's shortcuts, and whether they
    /// suit.
    fn shortcuts(&mut self, env: &Env, out: &mut Vec<Effect>) {
        let list = preset::shortcut_list(&preset::denis(), env.saved);
        if list.is_empty() {
            return;
        }
        let mut answer = tr("These are Denis's shortcuts — do they suit you?").to_owned();
        answer.push('\n');
        for (path, name, binding) in list {
            // Two named alike tell themselves apart by where they are.
            let name = if path.is_empty() {
                tr_window(&name).to_owned()
            } else if path.ends_with(name.as_str()) {
                tr_window(&path).to_owned()
            } else {
                format!("{} › {}", tr_window(&path), tr_window(&name))
            };
            answer.push_str(&format!("\n- {name}: {binding}"));
        }
        let asked = Turn {
            answer,
            preset: SHORTCUTS.into(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(asked, out);
    }

    /// The owner's shortcuts as a plan of their own.
    pub(super) fn use_shortcuts(&mut self, env: &Env, out: &mut Vec<Effect>) {
        let card = preset::shortcuts(&preset::denis(), env.saved);
        let answer = if card.is_some() {
            tr("Here they are. Untick any you'd rather keep as they are.")
        } else {
            tr("Your shortcuts already match Denis's.")
        };
        let plan = Turn {
            question: tr("Use these").into(),
            answer: answer.into(),
            cards: card.into_iter().collect(),
            preset: SHORTCUTS_PLAN.into(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(plan, out);
    }

    /// "Keep mine": the shortcuts stay.
    pub(super) fn keep_shortcuts(&mut self, env: &Env, out: &mut Vec<Effect>) {
        let kept = Turn {
            question: tr("Keep mine").into(),
            answer: tr("Your shortcuts stay as they are.").into(),
            preset: DONE.into(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(kept, out);
    }
}
