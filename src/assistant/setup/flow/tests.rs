//! The setup machine's paths as tables: each row an event sent, the
//! effects it asked for, and where the setup stood after. The pane, the
//! worker and the eval all move a setup through these same transitions.

use super::*;
use crate::assistant::facts::{Fact, State};
use crate::assistant::recordings::usage::Usage;
use crate::assistant::setup::about;
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
            if turn.error.is_some() {
                parts.push("unasked");
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

    /// Whether Continue on round `turn` (or its `skip`) would wait.
    fn waits(&self, turn: usize, skip: bool) -> bool {
        let context = self.context();
        let env = Env {
            context: &context,
            saved: &self.settings,
            now_ms: self.now,
            on_device: !self.remote,
            busy: self.busy,
            scene: false,
            changed: &|_, _| false,
        };
        self.flow.waits(turn, skip, &env)
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
            let (number, id) = (0, "yes".to_owned());
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
            "apply again: keep decided -> Done",
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

/// Round one's own words, written in its field: trimmed and capped, they
/// travel with the answers to the model's follow-ups and plan, are kept
/// with the setup, and come back prefilled when it returns, through
/// "Change answers" and to the model's look; cleared, they are gone.
#[test]
fn round_ones_own_words_travel_and_are_kept() {
    let words = "Пишу код на русском, диктую в Cursor";
    let stage_of = |effects: &[Effect]| {
        effects.iter().find_map(|effect| match effect {
            Effect::Ask { stage, .. } => Some(stage.clone()),
            _ => None,
        })
    };
    english(|| {
        let mut run = Run::new(true);
        run.send("begin", Event::Begin { since: None });
        let turn = run.last();
        // As the field writes it while typed: a draft in round one's turn.
        run.flow.turns[turn].about = format!("  {words}\n{}", " ".repeat(500));
        let asked = run.send("continue", Event::Answered { turn, skip: false });
        let follow = stage_of(&asked).expect("the follow-ups are asked");
        assert!(matches!(follow, Stage::Follow { .. }));
        assert_eq!(follow.about(), words);
        let kept = run.flow.kept.last().unwrap().clone();
        assert_eq!(kept.about, words, "kept from round one on");
        run.reply(&asked, false);
        let turn = run.last();
        let asked = run.send("skip", Event::Answered { turn, skip: true });
        let plan = stage_of(&asked).expect("the plan is asked");
        assert!(matches!(plan, Stage::Plan { .. }));
        assert_eq!(plan.about(), words);
        run.reply(&asked, false);
        run.decide("apply", applied());
        assert_eq!(run.flow.kept.last().unwrap().about, words);
        // Capped, however much was pasted.
        let mut long = Run::new(true);
        long.send("begin", Event::Begin { since: None });
        let turn = long.last();
        long.flow.turns[turn].about = "ы".repeat(about::MAX * 2);
        let asked = long.send("continue", Event::Answered { turn, skip: false });
        let stage = stage_of(&asked).unwrap();
        assert_eq!(stage.about().chars().count(), about::MAX);

        // Returning on this device: round one folded, the words prefilled,
        // kept through "Change answers".
        let kept = run.flow.kept.clone();
        let mut back = Run {
            remote: false,
            ..run.later(2)
        };
        let since = Since {
            last: since::last(&kept).cloned(),
            ..Since::default()
        };
        back.send("begin", Event::Begin { since: Some(since) });
        let turn = back.last();
        assert!(back.flow.turns[turn].folded);
        assert_eq!(back.flow.turns[turn].about, words);
        back.send("change", Event::Change { turn });
        assert_eq!(back.flow.turns[turn].about, words);
        // Cleared, the next record has none.
        back.flow.turns[turn].about.clear();
        back.send("continue", Event::Answered { turn, skip: false });
        assert_eq!(back.flow.kept.last().unwrap().about, "");

        // Returning on a model with the use read: the look reads them as
        // the last setup's own words, and round one comes back with them.
        let mut look = Run {
            flow: Flow::new(3, Vec::new(), kept.clone()),
            ..Run::new(true)
        };
        let since = Since {
            last: since::last(&kept).cloned(),
            usage: Some(Usage {
                dictations: 80,
                ..Usage::default()
            }),
            ..Since::default()
        };
        let asked = look.send("begin", Event::Begin { since: Some(since) });
        let Some(Stage::Look {
            facts,
            about: prefilled,
            ..
        }) = stage_of(&asked)
        else {
            panic!("a returning setup looks first");
        };
        assert!(
            facts.contains(&format!("{}{words}", about::THEN)),
            "{facts}"
        );
        assert_eq!(prefilled, words);
        look.reply(&asked, false);
        assert_eq!(look.flow.turns[look.last()].about, words);
        Vec::new()
    });
}

/// A model answering elsewhere is not asked, and nothing is taken as
/// answered for it: the round stays open, unkept, and Continue pressed
/// again once the model is free goes on. A round the app makes itself
/// doesn't wait. A request the worker turned down is a turn to try again
/// from, never a dead end.
#[test]
fn a_busy_model_keeps_the_round_open() {
    let trace = english(|| {
        let mut run = Run::new(true);
        run.busy = true;
        run.send("begin", Event::Begin { since: None });
        run.busy = false;
        run.send("begin", Event::Begin { since: None });
        run.busy = true;
        let turn = run.last();
        let open = run.flow.clone();
        assert!(run.waits(turn, false));
        run.send("continue", Event::Answered { turn, skip: false });
        assert_eq!(run.flow, open, "nothing answered, nothing kept");
        assert!(run.flow.kept.is_empty());
        run.busy = false;
        assert!(!run.waits(turn, false));
        let asked = run.send("continue", Event::Answered { turn, skip: false });
        assert_eq!(run.flow.kept.len(), 1);
        run.reply(&asked, false);
        run.busy = true;
        let turn = run.last();
        let open = run.flow.clone();
        assert!(run.waits(turn, true));
        run.send("skip", Event::Answered { turn, skip: true });
        assert_eq!(run.flow, open);
        run.busy = false;
        let asked = run.send("skip", Event::Answered { turn, skip: true });
        // The worker turned it down all the same.
        let Some(Effect::Ask { stage, .. }) = asked.last() else {
            panic!("the plan is a model's");
        };
        run.send("unasked", Event::Unasked(stage.clone()));
        let turn = run.last();
        run.busy = true;
        run.send("again", Event::Again { turn });
        run.busy = false;
        run.send("again", Event::Again { turn });
        // With no model to ask, the app's own round never waits.
        let mut own = Run::new(false);
        own.send("begin", Event::Begin { since: None });
        own.busy = true;
        let turn = own.last();
        assert!(!own.waits(turn, false));
        own.trace.clear();
        own.send("own continue", Event::Answered { turn, skip: false });
        run.trace.extend(own.trace);
        run.trace
    });
    assert_eq!(
        trace,
        [
            "begin:  -> None",
            "begin: push now round -> Basics",
            "continue:  -> Basics",
            "continue: answers, keep, ask follow -> Answered",
            "reply:  -> Follow",
            "skip:  -> Follow",
            "skip: answers, keep, ask plan -> Answered",
            "unasked: push unasked -> Stalled",
            "again:  -> Stalled",
            "again: ask plan -> Stalled",
            "own continue: answers, keep, push round -> Follow",
        ]
    );
}

/// A plan whose save failed decides nothing: the setup stays at its plan
/// and its record says no plan was decided; the "Try again" that goes
/// through is the one recorded.
#[test]
fn a_failed_save_decides_nothing() {
    let trace = english(|| {
        let mut run = Run::new(false);
        run.send("begin", Event::Begin { since: None });
        run.choose("language", "ru_tech");
        let turn = run.last();
        run.send("continue", Event::Answered { turn, skip: false });
        run.follow_ups();
        run.trace.clear();
        let failed = || Decision::Failed {
            why: "the file is locked".into(),
        };
        run.decide("apply fails", failed());
        let kept = run.flow.kept.last().unwrap();
        assert_eq!((kept.plan_ms, kept.applied.len()), (0, 0));
        run.decide("try again", applied());
        let kept = run.flow.kept.last().unwrap();
        assert_ne!(kept.plan_ms, 0);
        assert!(!kept.applied.is_empty(), "what went through is recorded");
        assert_eq!(run.flow.kept.len(), 1);
        run.trace
    });
    assert_eq!(
        trace,
        ["apply fails:  -> Plan", "try again: keep decided -> Done",]
    );
}

/// The record follows the plan's card: Undo takes what was applied out of
/// it, "Change anyway" after that or after "Not now" puts it in, and an
/// Undo that failed leaves it saying applied.
#[test]
fn the_record_follows_undo_and_change_anyway() {
    let trace = english(|| {
        let mut run = Run::new(false);
        run.send("begin", Event::Begin { since: None });
        run.choose("language", "ru_tech");
        let turn = run.last();
        run.send("continue", Event::Answered { turn, skip: false });
        run.follow_ups();
        run.trace.clear();
        let applied_lines = |run: &Run| run.flow.kept.last().unwrap().applied.len();
        run.decide("apply", applied());
        let whole = applied_lines(&run);
        assert_ne!(whole, 0);
        run.decide("undo", Decision::Undone);
        assert_eq!(applied_lines(&run), 0, "undone, it no longer says applied");
        run.decide("change anyway", applied());
        assert_eq!(applied_lines(&run), whole);
        let why = "changed by hand since".into();
        run.decide("undo fails", Decision::Failed { why });
        assert_eq!(applied_lines(&run), whole);
        run.decide("not now", Decision::Declined);
        assert_eq!(applied_lines(&run), 0);
        run.decide("change anyway", applied());
        assert_eq!(applied_lines(&run), whole);
        assert_eq!(run.flow.kept.len(), 1, "one setup, one record throughout");
        run.trace
    });
    assert_eq!(
        trace,
        [
            "apply: keep decided -> Done",
            "undo: keep decided -> Done",
            "change anyway: keep decided -> Done",
            "undo fails:  -> Done",
            "not now: keep decided -> Done",
            "change anyway: keep decided -> Done",
        ]
    );
}

/// The preset's shortcuts come once its plan was applied or put off: a
/// save that failed stays at the plan, and its "Try again" leads on. Undo
/// after that asks nothing twice.
#[test]
fn the_preset_stays_at_a_plan_that_failed() {
    let trace = english(|| {
        let mut run = Run::new(false);
        run.settings = AppSettings::default();
        run.settings.transcription.language = "ru".into();
        let question = String::new();
        run.send("preset", Event::Preset { question });
        let why = "the file is locked".into();
        run.decide("apply fails", Decision::Failed { why });
        run.decide("try again", applied());
        run.flow.turns[0].cards[0].decision = Decision::Undone;
        run.send("undo", Event::Decided { turn: 0, card: 0 });
        run.trace
    });
    assert_eq!(
        trace,
        [
            "preset: push preset settings -> PresetPlan",
            "apply fails:  -> PresetPlan",
            "try again: push preset shortcuts -> Shortcuts",
            "undo:  -> Shortcuts",
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
