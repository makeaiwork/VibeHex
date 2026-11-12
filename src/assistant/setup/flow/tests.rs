//! The setup machine's paths as tables: each row an event sent, the
//! effects it asked for, and where the setup stood after. The pane, the
//! worker and the eval all move a setup through these same transitions.

use super::*;
use crate::assistant::facts::{Fact, State};
use crate::assistant::recordings::usage::Usage;
use crate::desktop::i18n::speaking;

/// A setup run by hand: the sample install, a clock, and what happened.
struct Run {
    settings: AppSettings,
    state: State,
    flow: Flow,
    remote: bool,
    busy: bool,
    now: u64,
    trace: Vec<String>,
}

/// What an effect is, without the counts that differ by OS.
fn kind(effect: &Effect) -> String {
    match effect {
        Effect::Push(turn) if !turn.preset.is_empty() => format!("push preset {}", turn.preset),
        Effect::Push(turn) => {
            let mut parts = Vec::new();
            if !turn.now.is_empty() {
                parts.push("now");
            }
            if !turn.since.is_empty() {
                parts.push("since");
            }
            if !turn.asks.is_empty() {
                parts.push(if turn.folded { "round folded" } else { "round" });
            }
            if !turn.cards.is_empty() {
                parts.push("plan");
            } else if !turn.answer.is_empty() {
                parts.push("words");
            }
            format!("push {}", parts.join(" "))
        }
        Effect::Asks { .. } => "answers".into(),
        Effect::Join { .. } => "join".into(),
        Effect::Unfold { .. } => "unfold".into(),
        Effect::Keep(setup) if setup.plan_ms != 0 => "keep decided".into(),
        Effect::Keep(_) => "keep".into(),
        Effect::Ask { stage, .. } => match stage {
            Stage::Look { .. } => "ask look".into(),
            Stage::Follow { .. } => "ask follow".into(),
            Stage::Plan { .. } => "ask plan".into(),
            _ => "ask".into(),
        },
    }
}

fn stands(flow: &Flow) -> &'static str {
    match step(&flow.turns, &flow.kept) {
        Step::None => "None",
        Step::Looking => "Looking",
        Step::Basics { folded: true, .. } => "Basics folded",
        Step::Basics { .. } => "Basics",
        Step::Follow { .. } => "Follow",
        Step::Answered { .. } => "Answered",
        Step::Stalled { .. } => "Stalled",
        Step::Plan { .. } => "Plan",
        Step::PresetPlan { .. } => "PresetPlan",
        Step::Shortcuts { .. } => "Shortcuts",
        Step::ShortcutsPlan { .. } => "ShortcutsPlan",
        Step::Done => "Done",
    }
}

impl Run {
    /// The sample install on a Mac, asking a `remote` model or none.
    fn new(remote: bool) -> Self {
        let mut settings = AppSettings::default();
        crate::assistant::setup::sample(&mut settings);
        Self {
            settings,
            state: State {
                os: "macos",
                installed: Fact::Known(vec!["orukeet".into(), "parakeet_v3".into()]),
                codex_signed_in: Fact::Known(true),
                opencode_running: Fact::Known(false),
                ..State::default()
            },
            flow: Flow::new(1, Vec::new(), Vec::new()),
            remote,
            busy: false,
            now: 1_800_000_000_000,
            trace: Vec::new(),
        }
    }

    /// A later conversation on the same install, with the setups kept.
    fn later(self, conversation: u32) -> Self {
        Self {
            flow: Flow::new(conversation, Vec::new(), self.flow.kept.clone()),
            now: self.now + 20 * 86_400_000,
            ..self
        }
    }

    fn context(&self) -> Context {
        Context {
            settings: self.settings.clone(),
            state: self.state.clone(),
        }
    }

    /// Sends `event` and writes down what came of it; rows alike in a row
    /// (a second and third follow-up round) are one row.
    fn send(&mut self, name: &str, event: Event) -> Vec<Effect> {
        let context = self.context();
        self.now += 1_000;
        let env = Env {
            context: &context,
            saved: &self.settings,
            now_ms: self.now,
            on_device: !self.remote,
            busy: self.busy,
            scene: false,
            changed: &|_, _| false,
        };
        let (flow, effects) = std::mem::take(&mut self.flow).on(event, &env);
        self.flow = flow;
        let kinds: Vec<String> = effects.iter().map(kind).collect();
        self.wrote(format!(
            "{name}: {} -> {}",
            kinds.join(", "),
            stands(&self.flow)
        ));
        effects
    }

    fn wrote(&mut self, line: String) {
        if self.trace.last() != Some(&line) {
            self.trace.push(line);
        }
    }

    fn last(&self) -> usize {
        self.flow.turns.len() - 1
    }

    /// The model's turn for what `effects` asked, as the worker shapes it
    /// (`replied`): the follow-ups as offered, the app's own plan; or none
    /// of it when the model `failed`.
    fn reply(&mut self, effects: &[Effect], failed: bool) {
        let stage = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Ask { stage, .. } => Some(stage.clone()),
                _ => None,
            })
            .expect("a model was asked");
        let context = self.context();
        self.now += 1_000;
        let mut turn = Turn {
            created_ms: self.now,
            model: "a model".into(),
            ..Turn::default()
        };
        match &stage {
            _ if failed => turn.error = Some("no answer".into()),
            Stage::Follow { candidates, .. } => turn.asks = candidates.clone(),
            Stage::Plan { answers, terms, .. } => {
                let card = plan::with_terms(
                    Vec::new(),
                    answers,
                    terms,
                    &context.settings,
                    &context.state,
                );
                turn.cards = card.into_iter().collect();
            }
            _ => {}
        }
        replied(&stage, &mut turn, !failed, Some(&context));
        self.flow.turns.push(turn);
        let said = if failed { "no reply" } else { "reply" };
        self.wrote(format!("{said}:  -> {}", stands(&self.flow)));
    }

    /// Presses option `id` of question `ask` in the open round.
    fn choose(&mut self, ask: &str, id: &str) {
        let turn = self.last();
        let asks = &self.flow.turns[turn].asks;
        let number = asks.iter().position(|known| known.id == ask).unwrap();
        let id = id.to_owned();
        self.send("pick", Event::Pick { turn, number, id });
    }

    /// Answers every follow-up round: Yes to its first question, then
    /// Continue, until the setup is past them. However many rounds this
    /// OS has, the table holds the first press and the last Continue.
    fn follow_ups(&mut self) {
        let mark = self.trace.len();
        for round in 0.. {
            let turn = self.last();
            if !matches!(step(&self.flow.turns, &self.flow.kept), Step::Follow { .. }) {
                break;
            }
            assert!(
                round < follow::ROUNDS,
                "more rounds than {}",
                follow::ROUNDS
            );
            let (number, id) = (0, follow::YES.to_owned());
            self.send("pick", Event::Pick { turn, number, id });
            self.send("continue", Event::Answered { turn, skip: false });
        }
        if self.trace.len() > mark + 2 {
            let last = self.trace.pop().unwrap();
            self.trace.truncate(mark + 1);
            self.trace.push(last);
        }
    }

    /// The plan applied or put off, as the pane's save leaves its card.
    fn decide(&mut self, name: &str, decision: Decision) -> Vec<Effect> {
        let turn = self.last();
        self.flow.turns[turn].cards[0].decision = decision;
        self.send(name, Event::Decided { turn, card: 0 })
    }
}

fn applied() -> Decision {
    Decision::Applied {
        before: Default::default(),
        after: Default::default(),
        at_ms: 0,
    }
}

fn english(run: impl FnOnce() -> Vec<String>) -> Vec<String> {
    speaking(Some("en"), run)
}

/// A first setup with no model to ask: the app's own rounds and plan.
#[test]
fn a_first_setup_on_this_device() {
    let trace = english(|| {
        let mut run = Run::new(false);
        run.send("begin", Event::Begin { since: None });
        run.choose("language", "ru_tech");
        let turn = run.last();
        run.send("continue", Event::Answered { turn, skip: false });
        run.follow_ups();
        run.decide("apply", applied());
        run.decide("apply again", applied());
        run.trace
    });
    assert_eq!(
        trace,
        [
            "begin: push now round -> Basics",
            "pick: answers -> Basics",
            "continue: answers, keep, push round -> Follow",
            "pick: answers -> Follow",
            "continue: answers, keep, push plan -> Plan",
            "apply: keep decided -> Done",
            "apply again:  -> Done",
        ]
    );
}

/// A returning setup with no model: round one folded, what was answered
/// before not asked again, and "Change answers" to open it.
#[test]
fn a_returning_setup_on_this_device() {
    let trace = english(|| {
        let mut run = Run::new(false);
        run.send("begin", Event::Begin { since: None });
        let turn = run.last();
        run.send("continue", Event::Answered { turn, skip: false });
        run.follow_ups();
        run.decide("not now", Decision::Declined);
        let kept = run.flow.kept.clone();
        assert_eq!(kept.len(), 1, "one setup, kept once however often");
        let mut run = run.later(2);
        run.trace.push("later".into());
        let since = Since {
            last: since::last(&kept).cloned(),
            ..Since::default()
        };
        run.send("begin", Event::Begin { since: Some(since) });
        let turn = run.last();
        run.send("change", Event::Change { turn });
        run.send("continue", Event::Answered { turn, skip: false });
        run.follow_ups();
        assert_eq!(run.flow.kept.len(), 2);
        run.trace
    });
    assert_eq!(
        trace,
        [
            "begin: push now round -> Basics",
            "continue: answers, keep, push round -> Follow",
            "pick: answers -> Follow",
            "continue: answers, keep, push plan -> Plan",
            "not now: keep decided -> Done",
            "later",
            "begin: push now since round folded -> Basics folded",
            "change: unfold -> Basics",
            "continue: answers, keep, push round -> Follow",
            "pick: answers -> Follow",
            "continue: answers, keep, push plan -> Plan",
        ]
    );
}

/// A first setup on a remote model: the model words the follow-ups and
/// writes the plan; "Skip to the plan" goes past the follow-ups.
#[test]
fn a_first_setup_with_a_model() {
    let trace = english(|| {
        let mut run = Run::new(true);
        run.send("begin", Event::Begin { since: None });
        let turn = run.last();
        let asked = run.send("continue", Event::Answered { turn, skip: false });
        run.reply(&asked, false);
        let turn = run.last();
        let asked = run.send("skip", Event::Answered { turn, skip: true });
        run.reply(&asked, false);
        run.decide("apply", applied());
        run.trace
    });
    assert_eq!(
        trace,
        [
            "begin: push now round -> Basics",
            "continue: answers, keep, ask follow -> Answered",
            "reply:  -> Follow",
            "skip: answers, keep, ask plan -> Answered",
            "reply:  -> Plan",
            "apply: keep decided -> Done",
        ]
    );
}

/// A returning setup on a remote model, its use read: the model looks
/// first and round one comes after; a model that can't be asked leaves
/// round one at once; a turn that failed is asked again as itself.
#[test]
fn a_returning_setup_with_a_model() {
    let since = || Since {
        usage: Some(Usage {
            dictations: 80,
            meetings: 3,
            ..Usage::default()
        }),
        ..Since::default()
    };
    let trace = english(|| {
        let mut run = Run::new(true);
        let since_then = Some(since());
        let asked = run.send("begin", Event::Begin { since: since_then });
        run.reply(&asked, false);
        let turn = run.last();
        assert!(is_look(&run.flow.turns[turn]));
        let asked = run.send("look again", Event::LookAgain { turn });
        run.reply(&asked, false);
        let turn = run.last();
        let asked = run.send("continue", Event::Answered { turn, skip: false });
        run.reply(&asked, true);
        let turn = run.last();
        let asked = run.send("again", Event::Again { turn });
        run.reply(&asked, false);
        // The model couldn't be asked to look: round one at once.
        let mut run = Run {
            trace: run.trace,
            ..Run::new(true)
        };
        run.trace.push("unasked".into());
        let since_then = Some(since());
        let asked = run.send("begin", Event::Begin { since: since_then });
        let Some(Effect::Ask { stage, .. }) = asked.into_iter().last() else {
            panic!("the look is asked last");
        };
        run.send("unasked", Event::Unasked(stage));
        run.trace
    });
    assert_eq!(
        trace,
        [
            "begin: push now since, ask look -> Looking",
            "reply:  -> Basics folded",
            "look again: ask look -> Basics folded",
            "reply:  -> Basics folded",
            "continue: answers, keep, ask follow -> Answered",
            "no reply:  -> Stalled",
            "again: ask follow -> Stalled",
            "reply:  -> Follow",
            "unasked",
            "begin: push now since, ask look -> Looking",
            "unasked: push round folded -> Basics folded",
        ]
    );
}

/// While a model is answering, nothing more is asked of it: a setup
/// doesn't begin, and a round answered then stays answered with nothing
/// after it (today's behaviour, pinned).
#[test]
fn a_busy_model_is_not_asked() {
    let trace = english(|| {
        let mut run = Run::new(true);
        run.busy = true;
        run.send("begin", Event::Begin { since: None });
        run.busy = false;
        run.send("begin", Event::Begin { since: None });
        run.busy = true;
        let turn = run.last();
        run.send("continue", Event::Answered { turn, skip: false });
        run.send("again", Event::Again { turn });
        run.trace
    });
    assert_eq!(
        trace,
        [
            "begin:  -> None",
            "begin: push now round -> Basics",
            "continue: answers, keep -> Answered",
            "again:  -> Answered",
        ]
    );
}

/// The preset: its plan, then its shortcuts as a step of their own, and
/// only where it is offered.
#[test]
fn the_preset_then_its_shortcuts() {
    let trace = english(|| {
        // A fresh install: the sample one is the preset's own.
        let mut run = Run::new(false);
        run.settings = AppSettings::default();
        run.settings.transcription.language = "en".into();
        let preset = || Event::Preset {
            question: String::new(),
        };
        run.send("preset in English", preset());
        run.settings.transcription.language = "ru".into();
        run.send("preset", preset());
        run.decide("not now", Decision::Declined);
        // Decided again: the shortcuts aren't asked twice.
        run.send("not now again", Event::Decided { turn: 0, card: 0 });
        let mut kept = Run {
            settings: run.settings.clone(),
            flow: run.flow.clone(),
            ..Run::new(false)
        };
        let effects = run.send("use these", Event::UseShortcuts);
        assert_eq!(effects.len(), 1);
        kept.send("keep mine", Event::KeepShortcuts);
        run.trace.pop();
        run.trace.extend(kept.trace);
        run.trace
    });
    assert_eq!(
        trace,
        [
            "preset in English:  -> None",
            "preset: push preset settings -> PresetPlan",
            "not now: push preset shortcuts -> Shortcuts",
            "not now again:  -> Shortcuts",
            "keep mine: push preset done -> Done",
        ]
    );
}

/// An older setup, saved before setups were kept round by round (no
/// conversation, no start, its plan's time as its own), still counts as
/// that plan decided, and still answers what it answered.
#[test]
fn an_older_saved_setup_still_loads() {
    let saved = r#"{"at_ms": 5000, "answers": [{"id": "use", "chosen": ["meetings"]}]}"#;
    let old: Setup = serde_json::from_str(saved).unwrap();
    let plan = Turn {
        answers: old.answers.clone(),
        cards: vec![Card {
            kind: CardKind::Plan,
            ..Card::default()
        }],
        created_ms: 5_000,
        ..Turn::default()
    };
    let flow = Flow::new(3, vec![plan.clone()], vec![old.clone()]);
    assert_eq!(step(&flow.turns, &flow.kept), Step::Done);
    assert_eq!(step(&[plan], &[]), Step::Plan { turn: 0 });
    // A new setup is kept beside it, never in its place.
    let mut kept = vec![old];
    past::keep(
        &mut kept,
        Setup {
            conversation: 3,
            started_ms: 9_000,
            ..Setup::default()
        },
    );
    assert_eq!(kept.len(), 2);
}
