//! Runs a scenario: each step goes the way the pane sends it (the shared
//! routing, `intent::route` and `intent::reads`; the same `Question`; the
//! worker with its parse, gates, and repair round), offline with replies a
//! model is played as writing, or live on a real engine. What comes back
//! is the turn the pane would show, and what the model read.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use serde_json::Value;

use crate::app_settings::AppSettings;
use crate::assistant::changes::Context;
use crate::assistant::facts::State;
use crate::assistant::intent::{self, Reads, Route};
use crate::assistant::prompt::{self, Prompt, Room};
use crate::assistant::recordings::Sources;
use crate::assistant::setup::Stage;
use crate::assistant::store::Turn;
use crate::assistant::worker::{self, Earlier, Generate, Question, Recall, Searching};
use crate::dictation::cleanup_pieces::Streamed;
use crate::files::rewrite::Engine;

/// A made-up install a scenario plays on.
#[derive(Clone)]
pub struct Fixture {
    pub settings: AppSettings,
    pub state: State,
    /// The folder its recordings are read from, when it has any
    /// (`tests/fixtures/assistant/…`, synthetic).
    pub recordings: Option<PathBuf>,
    pub now_ms: u64,
}

impl Fixture {
    pub fn context(&self) -> Context {
        Context {
            settings: self.settings.clone(),
            state: self.state.clone(),
        }
    }
}

/// What the model was sent for a question.
#[derive(Clone, Debug, Default)]
pub struct Sent {
    pub instructions: String,
    pub input: String,
    /// It may search the web.
    pub web: bool,
}

impl Sent {
    pub fn all(&self) -> String {
        format!("{}\n{}", self.instructions, self.input)
    }
}

static SENT: Mutex<Option<Sent>> = Mutex::new(None);

/// The worker is one per process: its tests and the scenarios take turns
/// when a run keeps them in one (`cargo test`; nextest gives each its own).
pub static SERIAL: Mutex<()> = Mutex::new(());

/// The worker's word of what it sends, as it sends it.
pub fn sent(prompt: &Prompt, web: bool) {
    *SENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Sent {
        instructions: prompt.instructions.clone(),
        input: prompt.input.clone(),
        web,
    });
}

fn take_sent() -> Option<Sent> {
    SENT.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

/// What a step says.
pub enum Said {
    /// A message typed into the field.
    Message(&'static str),
    /// A setup step the pane sends itself: its words, and the stage built
    /// from the fixture by the app's own code.
    Stage(fn(&Fixture) -> (String, Stage)),
}

/// Where a check runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layer {
    Both,
    /// Only with the played reply: what the app does with exactly it.
    Offline,
    /// Only on a real engine: what a model makes of the prompt.
    Live,
}

pub type Verdict = Result<(), String>;

pub struct Check {
    pub name: String,
    pub layer: Layer,
    pub test: Box<dyn Fn(&Outcome) -> Verdict>,
}

/// One turn of a scenario.
pub struct Step {
    pub said: Said,
    /// The conversation's Web is on.
    pub web: bool,
    /// The replies a model is played as writing offline: the first, then
    /// the repair or rewrite rounds'; the last repeats.
    pub canned: Vec<&'static str>,
    pub checks: Vec<Check>,
}

/// A check of the app's own code, with no model in either layer.
pub struct AppCheck {
    pub name: String,
    pub test: Box<dyn Fn(&Fixture) -> Verdict>,
}

pub struct Scenario {
    pub id: &'static str,
    /// The problem it holds, as it was reported.
    pub about: &'static str,
    pub fixture: fn() -> Fixture,
    pub app: Vec<AppCheck>,
    pub steps: Vec<Step>,
}

/// What came of a step.
pub struct Outcome<'a> {
    pub fixture: &'a Fixture,
    /// A typed message's route.
    pub route: Option<Route>,
    /// It waits for the consent card: nothing was asked.
    pub consent: bool,
    /// What the model read; none when no model was asked.
    pub sent: Option<Sent>,
    pub turn: Turn,
}

/// How a scenario's replies come.
pub enum Mode {
    Canned,
    Live(Box<Engine>),
}

impl Mode {
    fn layer(&self) -> Layer {
        match self {
            Self::Canned => Layer::Offline,
            Self::Live(_) => Layer::Live,
        }
    }
}

/// One step's results.
pub struct StepRan {
    pub said: String,
    pub route: String,
    pub checks: Vec<(String, Verdict)>,
    pub turn: Turn,
    pub asked: bool,
    pub first_event: Option<f32>,
    pub seconds: f32,
    pub prompt_chars: usize,
}

pub struct Ran {
    pub id: &'static str,
    pub about: &'static str,
    pub app: Vec<(String, Verdict)>,
    pub steps: Vec<StepRan>,
}

impl Ran {
    pub fn failures(&self) -> Vec<String> {
        let app = self.app.iter().map(|check| (0, check));
        let steps = self
            .steps
            .iter()
            .enumerate()
            .flat_map(|(index, step)| step.checks.iter().map(move |check| (index + 1, check)));
        app.chain(steps)
            .filter_map(|(step, (name, verdict))| {
                let why = verdict.as_ref().err()?;
                Some(match step {
                    0 => format!("{}: {name}: {why}", self.id),
                    step => format!("{} step {step}: {name}: {why}", self.id),
                })
            })
            .collect()
    }

    pub fn passed(&self) -> bool {
        self.failures().is_empty()
    }
}

/// How long one answer may take: live, with its repair round; played, the
/// worker's own time.
const LIVE: Duration = Duration::from_secs(300);
const PLAYED: Duration = Duration::from_secs(30);

/// The earlier turns as the pane sends them (`composer::ask_with`).
fn earlier(turns: &[Turn]) -> Vec<Earlier> {
    turns
        .iter()
        .filter(|turn| turn.error.is_none() && !turn.answer.trim().is_empty())
        .map(|turn| Earlier {
            question: turn.question.clone(),
            answer: turn.answer.clone(),
            changes: prompt::changes_line(&turn.cards, &turn.refused),
            recordings: turn.read_recordings(),
        })
        .collect()
}

/// What a step asks: the question, or why nothing is asked.
enum Asks {
    Question(Box<Question>, Option<Route>),
    /// The preset's plan is the app's own: no model.
    Preset,
    Consent(Route),
}

fn asks(fixture: &Fixture, turns: &[Turn], step: &Step) -> Asks {
    let question = |text: String, recall, setup| {
        Box::new(Question {
            conversation: 1,
            question: text,
            earlier: earlier(turns),
            recall,
            changes: Some(fixture.context()),
            setup,
            web: step.web,
        })
    };
    match &step.said {
        Said::Stage(build) => {
            let (text, stage) = build(fixture);
            Asks::Question(question(text, Recall::Guide, stage), None)
        }
        Said::Message(text) => {
            let before: Vec<&str> = turns.iter().map(|turn| turn.question.as_str()).collect();
            let route = intent::route(text, &before, fixture.now_ms);
            let (recall, stage) = match route.clone() {
                Route::Preset => return Asks::Preset,
                Route::Setup => (Recall::Guide, Stage::Ask),
                Route::Releases(asked) => (Recall::Releases(asked), Stage::None),
                Route::Guide => (Recall::Guide, Stage::None),
                Route::Recordings(intent) => {
                    let sources = fixture.recordings.as_deref().map(Sources::at);
                    match intent::reads(intent, &fixture.settings, sources, fixture.now_ms) {
                        Reads::Recall(recall) => (recall, Stage::None),
                        Reads::Consent => return Asks::Consent(route),
                    }
                }
            };
            Asks::Question(question((*text).to_owned(), recall, stage), Some(route))
        }
    }
}

/// Starts the worker on `question`: a real engine, or the played replies
/// (as a remote model: one repair round, and the web when Web is on).
fn start(question: Question, step: &Step, mode: &Mode) -> bool {
    match mode {
        Mode::Live(engine) => worker::ask(question, (**engine).clone()),
        Mode::Canned => {
            let replies: VecDeque<String> = step.canned.iter().map(|&reply| reply.into()).collect();
            let replies = Arc::new(Mutex::new(replies));
            let next = move || {
                let mut replies = replies
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                match replies.len() {
                    0 => String::new(),
                    1 => replies[0].clone(),
                    _ => replies.pop_front().unwrap_or_default(),
                }
            };
            let again = next.clone();
            let generate: Generate = Box::new(move |_, _, _, _| Ok(next()));
            // A played search: one query, as Codex reports it, then the reply.
            let search = (question.web && question.setup == Stage::None).then(|| {
                let played = move |_: &str,
                                   _: &str,
                                   _: &AtomicBool,
                                   _: Option<Streamed>,
                                   seen: &dyn Fn(&Value, bool)|
                      -> Result<String> {
                    let item = serde_json::json!({
                        "id": "played",
                        "query": "played search",
                        "action": {"type": "search", "query": "played search"},
                    });
                    seen(&item, false);
                    seen(&item, true);
                    Ok(again())
                };
                Box::new(played) as Searching
            });
            worker::start_with(
                question,
                Room::Large,
                "played".into(),
                (generate, search),
                (false, None),
            )
        }
    }
}

/// Waits for the worker's turn, timing the first thing the pane would
/// show (a search step, a web search, the first words).
fn wait(patience: Duration) -> Option<(Turn, Option<f32>, f32)> {
    let started = Instant::now();
    let mut first = None;
    while started.elapsed() < patience {
        if first.is_none()
            && worker::answering().is_some_and(|answering| {
                !answering.draft.is_empty()
                    || !answering.steps.is_empty()
                    || !answering.web.is_empty()
                    || answering.cards_coming
            })
        {
            first = Some(started.elapsed().as_secs_f32());
        }
        if let Some((_, turn)) = worker::take_finished().pop() {
            return Some((turn, first, started.elapsed().as_secs_f32()));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    worker::stop();
    None
}

/// With `HEX_ASSISTANT_EVAL_DUMP` naming a folder, what each step sent is
/// written there, one file a step in the order asked: to read what a model
/// was told when a scenario fails.
fn dump(said: &str, sent: Option<&Sent>) {
    static STEP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let (Some(folder), Some(sent)) = (std::env::var_os("HEX_ASSISTANT_EVAL_DUMP"), sent) else {
        return;
    };
    let step = STEP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let folder = std::path::PathBuf::from(folder);
    let _ = std::fs::create_dir_all(&folder);
    let text = format!(
        "said: {said}\nweb: {}\n\n== instructions\n{}\n\n== input\n{}\n",
        sent.web, sent.instructions, sent.input
    );
    let _ = std::fs::write(folder.join(format!("{step:02}.txt")), text);
}

fn run_step(fixture: &Fixture, turns: &[Turn], step: &Step, mode: &Mode) -> StepRan {
    let said = match &step.said {
        Said::Message(text) => (*text).to_owned(),
        Said::Stage(build) => build(fixture).0,
    };
    let mut ran = StepRan {
        said,
        route: String::new(),
        checks: Vec::new(),
        turn: Turn::default(),
        asked: false,
        first_event: None,
        seconds: 0.0,
        prompt_chars: 0,
    };
    let mut outcome = Outcome {
        fixture,
        route: None,
        consent: false,
        sent: None,
        turn: Turn::default(),
    };
    match asks(fixture, turns, step) {
        Asks::Preset => outcome.route = Some(Route::Preset),
        Asks::Consent(route) => {
            outcome.route = Some(route);
            outcome.consent = true;
        }
        Asks::Question(question, route) => {
            outcome.route = route;
            take_sent();
            let patience = match mode {
                Mode::Canned => PLAYED,
                Mode::Live(_) => LIVE,
            };
            let asked_at = Instant::now();
            // Another answer may still be finishing.
            let mut started = start((*question).clone(), step, mode);
            while !started && asked_at.elapsed() < PLAYED {
                std::thread::sleep(Duration::from_millis(50));
                started = start((*question).clone(), step, mode);
            }
            ran.asked = started;
            match started.then(|| wait(patience)).flatten() {
                Some((turn, first, seconds)) => {
                    outcome.turn = turn;
                    ran.first_event = first;
                    ran.seconds = seconds;
                }
                None => outcome.turn.error = Some("no answer in time".into()),
            }
            outcome.sent = take_sent();
            dump(&ran.said, outcome.sent.as_ref());
        }
    }
    ran.route = match &outcome.route {
        Some(Route::Preset) => "preset".into(),
        Some(Route::Setup) => "setup".into(),
        Some(Route::Releases(_)) => "releases".into(),
        Some(Route::Recordings(_)) if outcome.consent => "recordings (consent)".into(),
        Some(Route::Recordings(_)) => "recordings".into(),
        Some(Route::Guide) => "guide".into(),
        None => "setup step".into(),
    };
    ran.prompt_chars = outcome.sent.as_ref().map_or(0, |sent| sent.all().len());
    let layer = mode.layer();
    for check in super::checks::always()
        .iter()
        .chain(&step.checks)
        .filter(|check| check.layer == Layer::Both || check.layer == layer)
    {
        ran.checks
            .push((check.name.clone(), (check.test)(&outcome)));
    }
    ran.turn = outcome.turn;
    ran
}

/// Runs `scenario` in `mode`: its app checks, then its steps as one
/// conversation.
pub fn run(scenario: &Scenario, mode: &Mode) -> Ran {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture = (scenario.fixture)();
    // The app's own words (setup's questions, a plan's reasons) in
    // Assistant's language for this install, as the pane makes them.
    let app = crate::assistant::language::within(&fixture.settings, || {
        scenario
            .app
            .iter()
            .map(|check| (check.name.clone(), (check.test)(&fixture)))
            .collect()
    });
    let mut turns: Vec<Turn> = Vec::new();
    let mut steps = Vec::new();
    for step in &scenario.steps {
        let ran = run_step(&fixture, &turns, step, mode);
        if ran.asked {
            turns.push(ran.turn.clone());
        }
        steps.push(ran);
    }
    Ran {
        id: scenario.id,
        about: scenario.about,
        app,
        steps,
    }
}
