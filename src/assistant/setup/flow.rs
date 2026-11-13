//! Guided setup as one state machine: "Set VibeHex up for me", first and
//! returning, and "Set me up like Denis".
//!
//! [`Flow`] is the setup running in one conversation: where it stands
//! ([`Step`]: what is set now, round one with the language and the two
//! engine questions, the follow-up rounds, the plan, the preset's
//! shortcuts, done) and everything gathered so far, which is the
//! conversation's own turns (what was looked at, asked and answered, the
//! plan) and the setups kept (`past`: answered, declined, applied). Nothing
//! else remembers a setup, so one left half-way goes on from what is saved.
//!
//! [`Flow::on`] is the one transition: a state and an [`Event`] give the
//! next state and the [`Effect`]s to perform: a turn to add, answers to
//! write, the setup's record to keep, a model to ask. It reads no file and
//! asks no one: [`Env`] carries what it needs to know of the install. The
//! pane sends the person's presses and performs the effects; the worker
//! asks the model and shapes its reply with [`replied`]; the eval scenarios
//! send the same events.

mod preset;
mod record;

pub use record::BUSY;

use super::past::{self, Setup};
use super::since::{self, Candidate, Since};
use super::{Answer, Ask, Stage, follow, plan};
use crate::app_settings::AppSettings;
use crate::assistant::changes::{Card, CardKind, Context, Decision};
use crate::assistant::recordings::Step as Read;
use crate::assistant::store::Turn;
use crate::desktop::i18n::tr;

/// A preset turn's steps (`Turn::preset`).
pub const SETTINGS: &str = "settings";
/// Older conversations hold `shortcuts-plan` too: a plan card of
/// Denis's shortcuts, before they were a shortcuts card. It reads as done.
pub const SHORTCUTS: &str = "shortcuts";
pub const DONE: &str = "done";

/// Where a setup stands, read off the conversation's last turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    /// No setup is open here.
    None,
    /// What is set now is shown, and a model looks at what came since.
    Looking,
    /// Round one, the app's four questions: what VibeHex is for, the
    /// language, and where dictation and notes are worked on. `folded` for
    /// a returning setup, until "Change answers".
    Basics { turn: usize, folded: bool },
    /// A follow-up round, `round` of at most [`follow::ROUNDS`].
    Follow { turn: usize, round: usize },
    /// A round is answered and nothing came after it yet: a model is
    /// writing the next, or couldn't be asked.
    Answered { turn: usize },
    /// A model's turn for the answers brought neither questions nor a
    /// plan: it failed or was stopped, and can be asked again.
    Stalled { turn: usize },
    /// The plan waits to be applied or put off.
    Plan { turn: usize },
    /// The preset's plan waits.
    PresetPlan { turn: usize },
    /// The preset's shortcuts card waits: Use these, or Keep mine.
    Shortcuts { turn: usize },
    /// Decided, or nothing to decide.
    Done,
}

/// What a person, or the pane for them, does to a setup.
#[derive(Clone, Debug)]
pub enum Event {
    /// "Set VibeHex up for me"; `since` for a returning one.
    Begin { since: Option<Since> },
    /// A model couldn't be asked what an [`Effect::Ask`] wanted.
    Unasked(Stage),
    /// A chip pressed: option `id` of question `number` in round `turn`.
    Pick {
        turn: usize,
        number: usize,
        id: String,
    },
    /// "Change answers": a returning round one opens.
    Change { turn: usize },
    /// Continue, or "Skip to the plan".
    Answered { turn: usize, skip: bool },
    /// A card's decision changed: applied, put off, undone, or its save
    /// failed.
    Decided { turn: usize, card: usize },
    /// A model's turn for the answers is asked again.
    Again { turn: usize },
    /// A returning setup's opening is looked at again.
    LookAgain { turn: usize },
    /// "Set me up like Denis", typed as `question` or pressed.
    Preset { question: String },
}

/// What a transition asks to be done.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// A turn joins the conversation.
    Push(Box<Turn>),
    /// Round `turn`'s questions, as picked or answered.
    Asks { turn: usize, asks: Vec<Ask> },
    /// Questions answered in an earlier setup join round `turn`.
    Join { turn: usize, asks: Vec<Ask> },
    /// Round `turn` opens.
    Unfold { turn: usize },
    /// The setup's record, to keep with the conversations.
    Keep(Setup),
    /// A model is asked `line` at `stage`; if it can't be, the machine is
    /// told so ([`Event::Unasked`]).
    Ask { line: String, stage: Stage },
}

/// What a transition knows of the install and the moment.
#[derive(Clone, Copy)]
pub struct Env<'a> {
    /// The settings as being typed and what was read off this install.
    pub context: &'a Context,
    /// The settings as saved: what a kept setup is measured against.
    pub saved: &'a AppSettings,
    pub now_ms: u64,
    /// The app writes the follow-ups and the plan itself: a built-in
    /// model, a model that can't be reached now, or a scene.
    pub on_device: bool,
    /// No model can be reached now (OpenCode isn't running, Codex isn't
    /// installed or is signed out): the app writes setup itself, and its
    /// plan ends pointing at the Connect panel under it.
    pub unreachable: bool,
    /// A model is answering: nothing more can be asked now.
    pub busy: bool,
    /// A scene: nothing is asked again.
    pub scene: bool,
    /// Whether a release changed setting `key` after `at_ms`
    /// (`since::changed_since`).
    pub changed: &'a dyn Fn(&str, u64) -> bool,
}

/// The setup in one conversation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flow {
    /// The conversation it runs in; 0 before it has one.
    pub conversation: u32,
    /// The conversation's turns.
    pub turns: Vec<Turn>,
    /// The setups kept, oldest first.
    pub kept: Vec<Setup>,
}

/// Whether this round is round one, with its fixed questions.
pub fn first(asks: &[Ask]) -> bool {
    asks.iter().any(|ask| ask.id == "use")
}

/// Whether `turn` is a returning setup's look: what the app read, a
/// model's suggestions, and round one after them.
pub fn is_look(turn: &Turn) -> bool {
    turn.folded
        && first(&turn.asks)
        && turn
            .steps
            .iter()
            .any(|step| matches!(step, Read::Looked { .. }))
}

/// The turn a model's reply at `stage` makes of `turn`, which holds what
/// the worker read of it (`read`: its questions are in `turn.asks`; not
/// when it failed or was stopped). `context` is what the question was
/// asked against.
pub fn replied(stage: &Stage, turn: &mut Turn, read: bool, context: Option<&Context>) {
    turn.answers = stage.answers().to_vec();
    turn.follow = matches!(stage, Stage::Follow { .. });
    if read {
        if let Stage::Follow { before, .. } = stage {
            // One round at a time; the rest wait.
            let rest = turn.asks.split_off(turn.asks.len().min(follow::ROUND));
            turn.more = rest;
            // What was answered before joins the round, folded.
            turn.asks.extend(before.iter().cloned());
        }
        if *stage == Stage::Ask
            && let Some(context) = context
        {
            turn.now = super::now::now(&context.facts());
        }
    }
    // A returning setup's opening: what the app read, then round one
    // folded, however the suggestions went.
    if let Stage::Look {
        facts,
        asks,
        terms,
        step,
        about,
        ..
    } = stage
    {
        turn.steps = vec![step.clone()];
        turn.asks = asks.clone();
        turn.folded = true;
        turn.facts = facts.clone();
        turn.terms = terms.clone();
        turn.about = about.clone();
    }
}

/// The turns of the open setup, from its round one.
fn setup_turns(turns: &[Turn]) -> &[Turn] {
    let from = turns
        .iter()
        .rposition(|turn| first(&turn.asks))
        .unwrap_or(0);
    &turns[from..]
}

/// Whether the plan made at `plan_ms` is kept as decided.
fn decided(kept: &[Setup], plan_ms: u64) -> bool {
    kept.iter().any(|setup| {
        // An older record, kept only once decided, has no conversation.
        plan_ms != 0
            && (setup.plan_ms == plan_ms || (setup.conversation == 0 && setup.at_ms == plan_ms))
    })
}

/// Where the setup in a conversation of `turns` stands, with the setups
/// `kept`: what the pane draws.
pub fn step(turns: &[Turn], kept: &[Setup]) -> Step {
    let Some(turn) = turns.len().checked_sub(1) else {
        return Step::None;
    };
    let last = &turns[turn];
    // A plan whose save failed is still to decide.
    let open = |card: Option<&Card>| {
        card.is_some_and(|card| {
            matches!(card.decision, Decision::Proposed | Decision::Failed { .. })
        })
    };
    match last.preset.as_str() {
        "" => {}
        SETTINGS if open(last.cards.first()) => return Step::PresetPlan { turn },
        SHORTCUTS if open(last.cards.first()) => return Step::Shortcuts { turn },
        _ => return Step::Done,
    }
    if !last.asks.is_empty() {
        return if last.asks.iter().all(|ask| ask.done) {
            Step::Answered { turn }
        } else if first(&last.asks) {
            Step::Basics {
                turn,
                folded: last.folded,
            }
        } else {
            let round = setup_turns(turns)
                .iter()
                .filter(|turn| !turn.asks.is_empty() && !first(&turn.asks))
                .count();
            Step::Follow { turn, round }
        };
    }
    if !last.now.is_empty() {
        return Step::Looking;
    }
    if last.answers.is_empty() {
        return Step::None;
    }
    let plan = last.cards.iter().find(|card| card.kind == CardKind::Plan);
    match plan {
        Some(_) if !last.follow && !decided(kept, last.created_ms) => Step::Plan { turn },
        Some(_) => Step::Done,
        None if last.follow || last.error.is_some() || last.stopped => Step::Stalled { turn },
        None => Step::Done,
    }
}

impl Flow {
    pub fn new(conversation: u32, turns: Vec<Turn>, kept: Vec<Setup>) -> Self {
        Self {
            conversation,
            turns,
            kept,
        }
    }

    fn setup_turns(&self) -> &[Turn] {
        setup_turns(&self.turns)
    }

    /// Every question the setup asked, in order.
    pub fn asked(&self) -> Vec<Ask> {
        self.setup_turns()
            .iter()
            .flat_map(|turn| turn.asks.iter().cloned())
            .collect()
    }

    /// What was answered so far.
    pub fn answers(&self) -> Vec<Answer> {
        super::answers(&self.asked())
    }

    /// The terms the opening's look found Vocabulary lacks, for the plan.
    pub fn terms(&self) -> Vec<Candidate> {
        self.setup_turns()
            .first()
            .map(|turn| turn.terms.clone())
            .unwrap_or_default()
    }

    /// Round one's own words (`setup::about`), trimmed and capped; empty
    /// when none were written.
    pub fn about(&self) -> String {
        self.setup_turns()
            .first()
            .filter(|turn| first(&turn.asks))
            .map(|turn| super::about::clean(&turn.about))
            .unwrap_or_default()
    }

    /// What the opening knew came since the last setup, for the model.
    pub fn facts(&self) -> String {
        self.setup_turns()
            .first()
            .map(|turn| turn.facts.clone())
            .unwrap_or_default()
    }

    /// The follow-up rounds asked so far.
    fn rounds(&self) -> usize {
        self.setup_turns()
            .iter()
            .filter(|turn| !turn.asks.is_empty() && !first(&turn.asks))
            .count()
    }

    /// What the questions and the plan are checked against: the install as
    /// read, with what the setups kept by now answered and declined.
    fn context(&self, env: &Env) -> Context {
        let mut context = env.context.clone();
        context.state.answered = past::answered_before(&self.kept, &context.settings, env.changed);
        context.state.declined = past::declined(&self.kept);
        context
    }

    /// The transition: the state after `event`, and what to do for it.
    pub fn on(mut self, event: Event, env: &Env) -> (Self, Vec<Effect>) {
        let mut out = Vec::new();
        match event {
            Event::Begin { since } => self.begin(since, env, &mut out),
            Event::Unasked(stage) => self.unasked(stage, env, &mut out),
            Event::Pick { turn, number, id } => self.pick(turn, number, &id, env, &mut out),
            Event::Change { turn } => {
                if self.turns.get(turn).is_some() {
                    self.emit(Effect::Unfold { turn }, &mut out);
                }
            }
            Event::Answered { turn, skip } => self.answered(turn, skip, env, &mut out),
            Event::Decided { turn, card } => {
                self.plan_decided(turn, card, env, &mut out);
                self.preset_decided(turn, card, env, &mut out);
            }
            Event::Again { turn } => self.again(turn, env, &mut out),
            Event::LookAgain { turn } => self.look_again(turn, env, &mut out),
            Event::Preset { question } => self.preset(question, env, &mut out),
        }
        (self, out)
    }

    /// A turn joins the conversation.
    fn push(&mut self, turn: Turn, out: &mut Vec<Effect>) {
        self.emit(Effect::Push(Box::new(turn)), out);
    }

    /// Takes `effect` into the state and hands it out to be performed.
    fn emit(&mut self, effect: Effect, out: &mut Vec<Effect>) {
        match &effect {
            Effect::Push(turn) => self.turns.push((**turn).clone()),
            Effect::Asks { turn, asks } => {
                if let Some(slot) = self.turns.get_mut(*turn) {
                    slot.asks = asks.clone();
                }
            }
            Effect::Join { turn, asks } => {
                if let Some(slot) = self.turns.get_mut(*turn) {
                    slot.asks.extend(asks.iter().cloned());
                }
            }
            Effect::Unfold { turn } => {
                if let Some(slot) = self.turns.get_mut(*turn) {
                    slot.folded = false;
                }
            }
            Effect::Keep(setup) => past::keep(&mut self.kept, setup.clone()),
            Effect::Ask { .. } => {}
        }
        out.push(effect);
    }

    /// What is set now, and round one preselected from it. A returning
    /// setup says what came since and folds round one; on a remote model
    /// with the use read, the model looks first and round one comes after
    /// its suggestions.
    fn begin(&mut self, since: Option<Since>, env: &Env, out: &mut Vec<Effect>) {
        if env.busy {
            return;
        }
        let context = self.context(env);
        let spoken = super::Spoken::of(&context.settings, &context.state);
        let mut asks = super::questions_for(context.state.os, spoken);
        super::preselect(&mut asks, &context.settings, &context.state);
        if let Some(since) = &since {
            since::preselect(&mut asks, since);
            super::preselect(&mut asks, &context.settings, &context.state);
        }
        let shown = since.as_ref().map(since::paragraph).unwrap_or_default();
        let usage = since.as_ref().and_then(|since| since.usage.as_ref());
        let looked = usage
            .map(|usage| Read::Looked {
                dictations: usage.dictations,
                meetings: usage.meetings,
            })
            .filter(|_| !env.on_device);
        let now = super::now::now(&context.facts());
        // A returning setup's own words are the last setup's, to change.
        let about = since
            .as_ref()
            .and_then(|since| since.last.as_ref())
            .map(|last| last.about.clone())
            .unwrap_or_default();
        let round = Turn {
            asks,
            facts: since.as_ref().map(since::prompt).unwrap_or_default(),
            folded: since.is_some(),
            about,
            terms: usage.map(since::candidates).unwrap_or_default(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        let Some(step) = looked else {
            let opening = Turn {
                now,
                since: shown,
                ..round
            };
            return self.push(opening, out);
        };
        let opening = Turn {
            now,
            since: shown.clone(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(opening, out);
        let stage = Stage::Look {
            facts: round.facts,
            shown,
            asks: round.asks,
            terms: round.terms,
            step,
            about: round.about,
        };
        let line = String::new();
        self.emit(Effect::Ask { line, stage }, out);
    }

    /// A chip pressed: a single choice takes it, a multiple one toggles.
    /// What the person picked is theirs; an untouched question follows it
    /// (code with Russian is Russian with English tech terms).
    fn pick(&mut self, turn: usize, number: usize, id: &str, env: &Env, out: &mut Vec<Effect>) {
        let Some(mut asks) = self.turns.get(turn).map(|turn| turn.asks.clone()) else {
            return;
        };
        let follow_up = !first(&asks);
        let Some(ask) = asks.get_mut(number) else {
            return;
        };
        ask.touched = true;
        if ask.multi {
            match ask.chosen.iter().position(|chosen| chosen == id) {
                Some(index) => {
                    ask.chosen.remove(index);
                }
                None => ask.chosen.push(id.to_owned()),
            }
        } else if ask.chosen == [id] && follow_up {
            // A follow-up pressed again is unanswered again.
            ask.chosen.clear();
        } else {
            ask.chosen = vec![id.to_owned()];
        }
        if asks[number].id == "use" {
            let context = self.context(env);
            super::preselect(&mut asks, &context.settings, &context.state);
        }
        self.emit(Effect::Asks { turn, asks }, out);
    }

    /// The round is answered (or skipped): the next round, or the plan.
    fn answered(&mut self, turn: usize, skip: bool, env: &Env, out: &mut Vec<Effect>) {
        let Some(mut asks) = self.turns.get(turn).map(|turn| turn.asks.clone()) else {
            return;
        };
        if first(&asks)
            && asks
                .iter()
                .any(|ask| ask.id == "use" && ask.chosen.is_empty())
        {
            return;
        }
        let shown: Vec<bool> = asks.iter().map(|ask| super::shows(&asks, ask)).collect();
        for (ask, shows) in asks.iter_mut().zip(shown) {
            ask.done = true;
            // An answer from before stands, skipped or not.
            if !shows || (skip && !ask.touched && !ask.before) {
                ask.chosen.clear();
            }
        }
        let before = self.clone();
        let mut moved = Vec::new();
        self.emit(Effect::Asks { turn, asks }, &mut moved);
        // Kept at once: left half-way, or its plan put off, the answers
        // stay.
        self.keep(None, env, &mut moved);
        if skip {
            self.plan(env, &mut moved);
        } else {
            self.next(turn, env, &mut moved);
        }
        // What comes next is a model's, and it is answering elsewhere:
        // the round stays open, nothing answered or kept, to continue
        // again once it is free.
        if env.busy
            && moved
                .iter()
                .any(|effect| matches!(effect, Effect::Ask { .. }))
        {
            *self = before;
            return;
        }
        out.extend(moved);
    }

    /// Whether Continue on round `turn` (or its skip) waits for a model
    /// answering elsewhere: pressed now, it would do nothing.
    pub fn waits(&self, turn: usize, skip: bool, env: &Env) -> bool {
        let free = Env {
            busy: false,
            ..*env
        };
        let ask = |effect: &Effect| matches!(effect, Effect::Ask { .. });
        env.busy
            && self
                .clone()
                .on(Event::Answered { turn, skip }, &free)
                .1
                .iter()
                .any(ask)
    }

    /// After round `turn`: the next follow-up round, else the plan.
    fn next(&mut self, turn: usize, env: &Env, out: &mut Vec<Effect>) {
        let Some(round) = self.turns.get(turn) else {
            return;
        };
        let more = round.more.clone();
        let follow_up = !first(&round.asks);
        if !more.is_empty() && self.rounds() < follow::ROUNDS {
            return self.round(more, Vec::new(), env, out);
        }
        if follow_up || !more.is_empty() {
            return self.plan(env, out);
        }
        let answers = self.answers();
        let context = self.context(env);
        let candidates = follow::candidates(&answers, &context.settings, &context.state);
        // What was answered in an earlier setup isn't asked again: it joins
        // the round folded, as answered then, or, with nothing new to ask,
        // round one's answers.
        let before = follow::before(&answers, &context.settings, &context.state);
        if candidates.is_empty() {
            let asks = before
                .into_iter()
                .map(|ask| Ask { done: true, ..ask })
                .collect();
            self.emit(Effect::Join { turn, asks }, out);
            return self.plan(env, out);
        }
        if env.on_device {
            return self.round(candidates, before, env, out);
        }
        let line = super::answers_line(&self.asked(), Ask::label);
        let stage = Stage::Follow {
            answers,
            line: line.clone(),
            candidates,
            since: self.facts(),
            before,
            about: self.about(),
        };
        self.emit(Effect::Ask { line, stage }, out);
    }

    /// A follow-up round of the app's: the first [`follow::ROUND`] of
    /// `asks`, the rest for later, and what was answered `before`, folded.
    fn round(&mut self, mut asks: Vec<Ask>, before: Vec<Ask>, env: &Env, out: &mut Vec<Effect>) {
        let more = asks.split_off(asks.len().min(follow::ROUND));
        asks.extend(before);
        let round = Turn {
            asks,
            more,
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(round, out);
    }

    /// Every answer so far, to the plan: the app's own on a built-in
    /// model, else the model's on top of the app's rules.
    fn plan(&mut self, env: &Env, out: &mut Vec<Effect>) {
        let asked = self.asked();
        let answers = super::answers(&asked);
        let line = super::answers_line(&asked, Ask::label);
        if !env.on_device {
            let stage = Stage::Plan {
                answers,
                line: line.clone(),
                since: self.facts(),
                terms: self.terms(),
                about: self.about(),
            };
            return self.emit(Effect::Ask { line, stage }, out);
        }
        // On this device no model writes the plan, so none is asked how a
        // term sounds either: a term keeps what History has it heard as
        // (`setup::spellings` says why).
        let context = self.context(env);
        let card = plan::with_terms(
            Vec::new(),
            &answers,
            &self.terms(),
            &context.settings,
            &context.state,
        );
        let answer = if card.as_ref().is_some_and(|card| !card.items.is_empty()) {
            tr("Here is what fits how you work. Untick what you don't need.")
        } else {
            tr("Everything already fits how you work.")
        };
        let answer = match connect_line(&answers, env) {
            Some(line) => format!("{answer}\n\n{line}"),
            None => answer.to_owned(),
        };
        let settled = card.is_none();
        let plan = Turn {
            answer,
            answers,
            cards: card.into_iter().collect(),
            created_ms: env.now_ms,
            ..Turn::default()
        };
        self.push(plan, out);
        if settled {
            // Nothing to decide: the setup is finished as it is.
            self.keep(None, env, out);
        }
    }

    /// Asks an answers turn again: its follow-ups, or its plan. With no
    /// model to reach, or after a model failed twice in a row, the app
    /// writes them itself, so a setup never stalls for good.
    fn again(&mut self, turn: usize, env: &Env, out: &mut Vec<Effect>) {
        if env.busy || env.scene {
            return;
        }
        let Some(asked) = self.turns.get(turn) else {
            return;
        };
        let (answers, line) = (asked.answers.clone(), asked.question.clone());
        if env.on_device || self.failed_twice(turn) {
            let own = Env {
                on_device: true,
                ..*env
            };
            if asked.follow {
                let context = self.context(&own);
                let candidates = follow::candidates(&answers, &context.settings, &context.state);
                if !candidates.is_empty() {
                    let before = follow::before(&answers, &context.settings, &context.state);
                    return self.round(candidates, before, &own, out);
                }
            }
            return self.plan(&own, out);
        }
        let stage = if asked.follow {
            let context = self.context(env);
            Stage::Follow {
                candidates: follow::candidates(&answers, &context.settings, &context.state),
                before: follow::before(&answers, &context.settings, &context.state),
                answers,
                line: line.clone(),
                since: self.facts(),
                about: self.about(),
            }
        } else {
            Stage::Plan {
                answers,
                line: line.clone(),
                since: self.facts(),
                terms: self.terms(),
                about: self.about(),
            }
        };
        self.emit(Effect::Ask { line, stage }, out);
    }

    /// Whether turn `turn` and the one before it are both a model's failed
    /// try at the same stage: asked again once already.
    fn failed_twice(&self, turn: usize) -> bool {
        let failed = |at: usize| {
            self.turns
                .get(at)
                .filter(|turn| turn.error.is_some() && !turn.answers.is_empty())
                .map(|turn| turn.follow)
        };
        turn.checked_sub(1)
            .is_some_and(|before| failed(turn).is_some() && failed(turn) == failed(before))
    }

    /// Looks again at what came since: a returning setup's opening asked
    /// again, from the turn that showed it.
    fn look_again(&mut self, turn: usize, env: &Env, out: &mut Vec<Effect>) {
        if env.busy || env.scene {
            return;
        }
        let Some(look) = self.turns.get(turn) else {
            return;
        };
        let Some(step) = look.steps.first().cloned() else {
            return;
        };
        let shown = self
            .turns
            .iter()
            .rev()
            .find(|turn| !turn.since.is_empty())
            .map(|turn| turn.since.clone())
            .unwrap_or_default();
        let mut asks = look.asks.clone();
        for ask in &mut asks {
            ask.done = false;
        }
        let stage = Stage::Look {
            facts: look.facts.clone(),
            shown,
            asks,
            terms: look.terms.clone(),
            step,
            about: look.about.clone(),
        };
        let line = String::new();
        self.emit(Effect::Ask { line, stage }, out);
    }
}

/// The plan's last line when no model can be reached: questions need one
/// connected, Codex first, and Codex chosen here still needs signing in.
/// Assistant's own model never changes through a card.
fn connect_line(answers: &[super::Answer], env: &Env) -> Option<&'static str> {
    if !env.unreachable {
        return None;
    }
    let codex = answers.iter().any(|answer| {
        matches!(answer.id.as_str(), "cleanup" | "notes")
            && answer.chosen.iter().any(|chosen| chosen == "codex")
    });
    Some(if codex {
        tr(
            "To ask Assistant questions, connect a model below: Codex works best. You chose Codex, so it still needs signing in.",
        )
    } else {
        tr(
            "To ask Assistant questions, connect a model below: Codex works best if you have ChatGPT Plus or Pro.",
        )
    })
}

#[cfg(test)]
mod tests;
