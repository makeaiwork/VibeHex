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

use crate::app_settings::{AppSettings, AssistantSettings};
use crate::assistant::agent::access::Access;
use crate::assistant::agent::{self, Limits};
use crate::assistant::changes::Context;
use crate::assistant::facts::State;
use crate::assistant::intent::{self, Reads, Route};
use crate::assistant::prompt::{self, Prompt, Room};
use crate::assistant::recordings::{Allowed, Sources};
use crate::assistant::setup::Stage;
use crate::assistant::setup::past::Setup;
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
    /// The setups kept on it before, oldest first.
    pub setups: Vec<Setup>,
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

impl Outcome<'_> {
    /// The turn went through the agent's loop: its trace opens with the
    /// app's own pre-read.
    pub fn agent(&self) -> bool {
        !self.turn.trace.is_empty()
    }

    /// The steps of every search of the recordings the turn made: the
    /// router's one, or each an agent turn's model asked for.
    pub fn searched(&self) -> Vec<crate::assistant::recordings::Step> {
        let mut steps = self.turn.steps.clone();
        for did in &self.turn.trace {
            if let crate::assistant::agent::trace::Did::Recordings { steps: more } = did {
                steps.extend(more.iter().cloned());
            }
        }
        steps
    }

    /// Everything the model read over the turn's requests.
    pub fn read(&self) -> String {
        let all: Vec<String> = self.sent.iter().map(Sent::all).collect();
        all.join("\n")
    }
}

/// Every request of the turn being answered, in the order sent.
static SENT: Mutex<Vec<Sent>> = Mutex::new(Vec::new());

/// The worker is one per process: its tests and the scenarios take turns
/// when a run keeps them in one (`cargo test`; nextest gives each its own).
pub static SERIAL: Mutex<()> = Mutex::new(());

/// The worker's word of what it sends, as it sends it: one for each
/// request of a turn (its repair and rewrite rounds aside).
pub fn sent(prompt: &Prompt, web: bool) {
    SENT.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(Sent {
            instructions: prompt.instructions.clone(),
            input: prompt.input.clone(),
            web,
        });
}

/// The tools an agent turn's model called, as it named them, in order.
static CALLED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The agent loop's word of each call it was asked to run.
pub fn called(tool: &str) {
    CALLED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(tool.to_owned());
}

fn take_called() -> Vec<String> {
    std::mem::take(
        &mut *CALLED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    )
}

fn take_sent() -> Vec<Sent> {
    std::mem::take(&mut *SENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
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
    /// What the person says when an agent turn asks, mid-answer, whether
    /// their recordings may be read: yes, or not now.
    pub allows: bool,
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
    /// The consent card came up: before anything was asked, or under an
    /// agent turn's steps when its model called for the recordings.
    pub consent: bool,
    /// The tools an agent turn's model called, as it named them.
    pub called: Vec<String>,
    /// An earlier turn of the conversation held something of the
    /// person's own (`checks::holds`).
    pub held_before: bool,
    /// What the model read, a request each, in order; none when no model
    /// was asked.
    pub sent: Vec<Sent>,
    pub turn: Turn,
}

/// How a scenario's replies come.
pub enum Mode {
    /// Played replies, one request a turn, read for by the router.
    Canned,
    /// Played replies through the agent's loop (`assistant::agent`).
    Agent,
    Live(Box<Engine>),
}

impl Mode {
    fn layer(&self) -> Layer {
        match self {
            Self::Canned | Self::Agent => Layer::Offline,
            Self::Live(_) => Layer::Live,
        }
    }
}

/// Which of a step's checks a run holds it to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Held {
    /// Its own, and the ones every step is held to.
    All,
    /// Only the ones every step is held to (`checks::always`): for a
    /// scenario written for the router, played through the agent's loop.
    Always,
}

/// One step's results.
pub struct StepRan {
    pub said: String,
    pub route: String,
    pub checks: Vec<(String, Verdict)>,
    pub turn: Turn,
    pub asked: bool,
    pub first_event: Option<f32>,
    /// When the first thing the model asked to read showed as a step.
    pub first_read: Option<f32>,
    /// When the answer's first words showed.
    pub first_word: Option<f32>,
    pub seconds: f32,
    pub prompt_chars: usize,
    /// How many requests the turn made (its repair round aside).
    pub requests: usize,
    /// The tools its model called.
    pub called: Vec<String>,
    /// What it sent held something of the person's own.
    pub held: bool,
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
            access: Access::of(
                &fixture.settings,
                fixture.recordings.as_deref().map(Sources::at),
            ),
            now_ms: fixture.now_ms,
        })
    };
    match &step.said {
        Said::Stage(build) => {
            let (text, stage) = build(fixture);
            Asks::Question(question(text, Recall::Guide, stage), None)
        }
        Said::Message(text) => {
            let before: Vec<&str> = turns.iter().map(|turn| turn.question.as_str()).collect();
            let route = intent::route(text, &before, &fixture.settings, fixture.now_ms);
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
        Mode::Canned | Mode::Agent => {
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
                matches!(mode, Mode::Agent).then_some(Limits::REMOTE),
            )
        }
    }
}

/// What came of waiting for the worker's turn.
struct Waited {
    turn: Turn,
    /// When the first thing the pane would show came (a step, a web
    /// search, the first words).
    first_event: Option<f32>,
    first_read: Option<f32>,
    first_word: Option<f32>,
    seconds: f32,
    /// The consent card came up mid-answer.
    consent: bool,
}

/// Waits for the worker's turn, timing what the pane would show, and
/// answering an agent turn's consent card as the step's person does.
fn wait(patience: Duration, allows: bool) -> Option<Waited> {
    let started = Instant::now();
    let (mut first_event, mut first_read, mut first_word) = (None, None, None);
    let mut consent = false;
    while started.elapsed() < patience {
        let now = started.elapsed().as_secs_f32();
        if let Some(answering) = worker::answering() {
            // The app's own pre-read is the first line of every agent
            // turn; what the model asked for comes after it.
            let read = !answering.steps.is_empty() || answering.trace.len() > 1;
            if first_event.is_none()
                && (!answering.draft.is_empty()
                    || read
                    || !answering.web.is_empty()
                    || answering.cards_coming)
            {
                first_event = Some(now);
            }
            if first_read.is_none() && read {
                first_read = Some(now);
            }
            if first_word.is_none() && !answering.draft.is_empty() {
                first_word = Some(now);
            }
            if answering.asking {
                consent = true;
                // As the pane does: the setting first, then the leave.
                let yes = AssistantSettings {
                    read_recordings: Some(true),
                    ..AssistantSettings::default()
                };
                agent::live::decide(allows.then(|| Allowed::from_settings(&yes)).flatten());
            }
        }
        if let Some((_, turn)) = worker::take_finished().pop() {
            return Some(Waited {
                turn,
                first_event,
                first_read,
                first_word,
                seconds: started.elapsed().as_secs_f32(),
                consent,
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    worker::stop();
    None
}

/// With `HEX_ASSISTANT_EVAL_DUMP` naming a folder, what each step sent is
/// written there, one file a step in the order asked: to read what a model
/// was told when a scenario fails.
fn dump(said: &str, sent: &[Sent]) {
    static STEP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let Some(folder) = std::env::var_os("HEX_ASSISTANT_EVAL_DUMP").filter(|_| !sent.is_empty())
    else {
        return;
    };
    let step = STEP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let folder = std::path::PathBuf::from(folder);
    let _ = std::fs::create_dir_all(&folder);
    for (request, sent) in sent.iter().enumerate() {
        let text = format!(
            "said: {said}\nweb: {}\n\n== instructions\n{}\n\n== input\n{}\n",
            sent.web, sent.instructions, sent.input
        );
        // A turn's first request keeps the name it always had.
        let name = match request {
            0 => format!("{step:02}.txt"),
            request => format!("{step:02}-{}.txt", request + 1),
        };
        let _ = std::fs::write(folder.join(name), text);
    }
}

fn run_step(
    fixture: &Fixture,
    turns: &[Turn],
    step: &Step,
    (mode, held, held_before): (&Mode, Held, bool),
) -> StepRan {
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
        first_read: None,
        first_word: None,
        seconds: 0.0,
        prompt_chars: 0,
        requests: 0,
        called: Vec::new(),
        held: false,
    };
    let mut outcome = Outcome {
        fixture,
        route: None,
        consent: false,
        called: Vec::new(),
        held_before,
        sent: Vec::new(),
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
            take_called();
            let patience = match mode {
                Mode::Canned | Mode::Agent => PLAYED,
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
            match started.then(|| wait(patience, step.allows)).flatten() {
                Some(waited) => {
                    outcome.turn = waited.turn;
                    outcome.consent = waited.consent;
                    ran.first_event = waited.first_event;
                    ran.first_read = waited.first_read;
                    ran.first_word = waited.first_word;
                    ran.seconds = waited.seconds;
                }
                None => outcome.turn.error = Some("no answer in time".into()),
            }
            outcome.sent = take_sent();
            outcome.called = take_called();
            dump(&ran.said, &outcome.sent);
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
    ran.prompt_chars = outcome.sent.iter().map(|sent| sent.all().len()).sum();
    ran.requests = outcome.sent.len();
    ran.called = outcome.called.clone();
    ran.held = outcome
        .sent
        .iter()
        .any(|sent| !super::checks::holds(&sent.input, outcome.agent()).is_empty());
    let layer = mode.layer();
    let own: &[Check] = match held {
        Held::All => &step.checks,
        Held::Always => &[],
    };
    for check in super::checks::always()
        .iter()
        .chain(own)
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
    run_held(scenario, mode, Held::All)
}

/// [`run`], holding each step to `held`.
pub fn run_held(scenario: &Scenario, mode: &Mode, held: Held) -> Ran {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Each scenario is a fresh run of the app: nobody has said no yet.
    agent::live::forget();
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
    let mut held_before = false;
    for step in &scenario.steps {
        let ran = run_step(&fixture, &turns, step, (mode, held, held_before));
        if ran.asked {
            turns.push(ran.turn.clone());
        }
        held_before |= ran.held;
        steps.push(ran);
    }
    Ran {
        id: scenario.id,
        about: scenario.about,
        app,
        steps,
    }
}
