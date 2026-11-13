//! Assistant's answers run on a thread of their own, never behind a Files
//! job: one question at a time, its reply streaming into a draft the pane
//! reads. A stop keeps what came; a failure becomes the turn's error. The
//! pane takes finished turns into its store. With Web on, a Codex answer
//! may search the web, each search reaching the pane as a step.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use serde_json::Value;

use super::agent::{self, access::Access, trace::Did, trace::Doing};
use super::changes;
use super::intent::Intent;
use super::prompt::{self, Reach, Room};
use super::recordings::{self, Allowed, Budget, Found, Sources, Step};
use super::releases;
use super::settle::{Settled, settle};
use super::setup::{self, Stage};
use super::store::Turn;
use super::web::Web;
use crate::dictation::cleanup_pieces::Streamed;
use crate::dictation::cleanup_server::engine;
use crate::dictation::vocabulary::VocabularyEntry;
use crate::files::rewrite::Engine;

/// How often a growing draft reaches the pane.
const DRAFT_EVERY: Duration = Duration::from_millis(80);
/// How long a cloud model may stay silent after its turn starts (no word,
/// no reasoning, no search) before the turn is stopped and asked once
/// more; the engine's own deadline then counts from the new turn. A model
/// that is merely slow has said something by then.
const FIRST_EVENT: Duration = Duration::from_secs(75);
/// How often the watchdog looks.
const WATCH_EVERY: Duration = Duration::from_millis(50);

pub(super) type Generate =
    Box<dyn Fn(&str, &str, &AtomicBool, Option<Streamed>) -> Result<String> + Send + 'static>;
/// [`Generate`] with web search allowed; the last argument hears each of
/// Codex's `webSearch` items, and whether it is done.
pub(super) type Searching = Box<
    dyn Fn(&str, &str, &AtomicBool, Option<Streamed>, &dyn Fn(&Value, bool)) -> Result<String>
        + Send
        + 'static,
>;

/// A question for a conversation, with its earlier turns.
#[derive(Clone, Debug)]
pub struct Question {
    pub conversation: u32,
    pub question: String,
    /// Earlier turns, oldest first.
    pub earlier: Vec<Earlier>,
    /// What it may read besides the guide.
    pub recall: Recall,
    /// The settings it may suggest changes to, if it may.
    pub changes: Option<changes::Context>,
    /// Guided setup's questions or plan, or neither.
    pub setup: Stage,
    /// The conversation's Web is on: a Codex answer may search the web.
    pub web: bool,
    /// What an agent turn may read of the person's recordings when its
    /// model asks to (`agent::tools`); the router's own search carries its
    /// leave in `recall`.
    pub access: Access,
    /// When it was asked, for the times an agent turn's tools read
    /// (`yesterday`, `7d`).
    pub now_ms: u64,
}

/// An earlier question, its answer, and what became of its changes.
#[derive(Clone, Debug, Default)]
pub struct Earlier {
    pub question: String,
    pub answer: String,
    pub changes: String,
    /// Its answer was written from the person's recordings, so it may
    /// quote them: the web stays off for the rest of the conversation.
    pub recordings: bool,
}

/// What a question may read besides the guide.
#[derive(Clone, Debug, Default)]
pub enum Recall {
    #[default]
    Guide,
    /// About the person's recordings, which Assistant may not read.
    Declined,
    /// About the person's recordings, which it may search (boxed: it
    /// carries the question's intent and the Vocabulary).
    Search(Box<Search>),
    /// About what changed in VibeHex: its release notes.
    Releases(releases::Asked),
}

/// A search of the person's recordings, possible only with their leave.
#[derive(Clone, Debug)]
pub struct Search {
    pub allowed: Allowed,
    pub sources: Sources,
    pub intent: Intent,
    /// The person's Vocabulary, so a term is found however it was
    /// written; the search reads it, the model never does.
    pub vocabulary: Vec<VocabularyEntry>,
    pub now_ms: u64,
}

/// The question being answered and what has come so far.
#[derive(Clone, Debug)]
pub struct Answering {
    pub conversation: u32,
    pub question: String,
    /// The reply so far, without a half-written `Sources:` line.
    pub draft: String,
    pub started: Instant,
    /// The recordings search's steps so far.
    pub steps: Vec<Step>,
    /// Still searching the recordings.
    pub searching: bool,
    /// A change block has started: the feed shows "Preparing a change".
    pub cards_coming: bool,
    /// What it has searched on the web so far.
    pub web: Web,
    /// The model stayed silent past [`FIRST_EVENT`] and is asked again.
    pub retrying: bool,
    /// What an agent turn has read so far, in order (`agent::trace`).
    pub trace: Vec<Did>,
    /// What an agent turn is doing now.
    pub doing: Option<Doing>,
    /// An agent turn waits for the person to say whether their recordings
    /// may be read: the consent card shows under its steps.
    pub asking: bool,
}

#[derive(Default)]
struct State {
    answering: Option<Answering>,
    cancel: Option<Arc<AtomicBool>>,
    finished: Vec<(u32, Turn)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    answering: None,
    cancel: None,
    finished: Vec::new(),
});
type Waker = Arc<dyn Fn() + Send + Sync>;
static WAKER: Mutex<Option<Waker>> = Mutex::new(None);

fn state() -> MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Called on every change, so the window redraws.
pub fn set_waker(waker: impl Fn() + Send + Sync + 'static) {
    *WAKER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::new(waker));
}

fn changed(state: MutexGuard<'static, State>) {
    drop(state);
    let waker = WAKER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if let Some(waker) = waker {
        waker();
    }
}

/// Asks `engine`; false while another question is being answered.
pub fn ask(question: Question, engine: Engine) -> bool {
    let local = engine.is_local();
    let room = if local { Room::Small } else { Room::Large };
    let context = if local {
        engine::WIDE_CONTEXT_TOKENS
    } else {
        engine::CONTEXT_TOKENS
    };
    let model = engine.model_name();
    let search: Option<Searching> = searches(&question, &engine).then(|| {
        let engine = engine.clone();
        Box::new(
            move |instructions: &str,
                  input: &str,
                  cancel: &AtomicBool,
                  streamed,
                  seen: &dyn Fn(&Value, bool)| {
                engine.generate_searching(instructions, input, cancel, streamed, seen)
            },
        ) as Searching
    });
    // A cloud model's silence is a stall; a built-in model, OpenCode
    // (which answers whole) and the person's own server (reading a long
    // prompt on their hardware) may be silent a long while.
    let silence = engine.streams_from_the_cloud().then_some(FIRST_EVENT);
    let engine_kind = engine.clone();
    let generate: Generate = Box::new(move |instructions, input, cancel, streamed| {
        engine.generate(context, instructions, input, cancel, streamed)
    });
    let agent = agent::limits(
        &engine_kind,
        question
            .changes
            .as_ref()
            .map(|context| &context.settings.assistant),
    );
    start_with(
        question,
        room,
        model,
        (generate, search),
        (local, silence),
        agent,
    )
}

/// Whether `question`'s answer may search the web: its conversation's Web
/// is on, it runs on Codex, and it isn't guided setup's. What it reads may
/// still keep it off ([`offline`]).
fn searches(question: &Question, engine: &Engine) -> bool {
    question.web && engine.searches_web() && question.setup == Stage::None
}

/// Why an answer that may search the web stays off it: its prompt holds
/// the person's recordings (meetings, dictations, Files texts), and those
/// never go online, whatever the model is told. The settings it reads
/// carry no vocabulary, keys, or addresses, and may.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Offline {
    /// The question is about their recordings, its own or carried over
    /// from the one before: the web has nothing for it, and nothing is said.
    Recordings,
    /// An earlier answer quotes them; the question may need the web, so
    /// the steps say it stayed off.
    Earlier,
}

fn offline(question: &Question, found: Option<&Found>, vocabulary: bool) -> Option<Offline> {
    // Vocabulary entries are the person's own, as recordings are.
    if found.is_some() || vocabulary || matches!(question.recall, Recall::Declined) {
        Some(Offline::Recordings)
    } else if question.earlier.iter().any(|earlier| earlier.recordings) {
        Some(Offline::Earlier)
    } else {
        None
    }
}

#[cfg(test)]
fn start(question: Question, room: Room, model: String, generate: Generate, local: bool) -> bool {
    start_with(question, room, model, (generate, None), (local, None), None)
}

pub(super) fn start_with(
    question: Question,
    room: Room,
    model: String,
    (generate, search): (Generate, Option<Searching>),
    (local, silence): (bool, Option<Duration>),
    agent: Option<agent::Limits>,
) -> bool {
    let mut held = state();
    if held.answering.is_some() {
        return false;
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let started = Instant::now();
    held.answering = Some(Answering {
        conversation: question.conversation,
        // Setup's answers show as chips, never as a question.
        question: match question.setup {
            Stage::Plan { .. } | Stage::Follow { .. } | Stage::Look { .. } => String::new(),
            _ => question.question.clone(),
        },
        draft: String::new(),
        started,
        // A returning setup's opening says what the app read first.
        steps: match &question.setup {
            Stage::Look { step, .. } => vec![step.clone()],
            _ => Vec::new(),
        },
        searching: matches!(question.recall, Recall::Search(_)),
        cards_coming: false,
        web: Web::default(),
        retrying: false,
        trace: Vec::new(),
        doing: None,
        asking: false,
    });
    held.cancel = Some(cancel.clone());
    changed(held);
    let spawned = std::thread::Builder::new()
        .name("assistant".into())
        .spawn(move || {
            // Its own words (setup's questions, "Now:", a plan's reasons)
            // in Assistant's language, as the pane makes them.
            let settings = question
                .changes
                .as_ref()
                .map(|context| context.settings.clone());
            super::language::within_maybe(settings.as_ref(), || {
                // Setup's stages are one request each, whatever the engine.
                let agent = agent.filter(|_| question.setup == Stage::None);
                let asker = Asker {
                    generate: &generate,
                    search: search.as_ref(),
                    cancel: &cancel,
                    silence,
                    // One more for a turn of several requests.
                    retries: Cell::new(if agent.is_some() { 2 } else { 1 }),
                    reads: agent.is_some(),
                };
                let answered = match agent {
                    Some(limits) => agent::run(&question, &asker, limits),
                    None => one_shot(&question, room, &asker, local),
                };
                finish(&question, &model, started, answered, &cancel);
                if local {
                    engine::narrow();
                }
            });
        });
    if let Err(error) = spawned {
        let mut state = state();
        state.answering = None;
        state.cancel = None;
        state.finished.push((
            0,
            Turn {
                error: Some(format!("couldn't start: {error}")),
                ..Turn::default()
            },
        ));
        changed(state);
        return false;
    }
    true
}

/// How a turn asks its model: the engine's plain call and, when the answer
/// may search the web, its searching one; the person's Stop; and how long
/// a cloud model may stay silent before it is asked again.
pub(super) struct Asker<'a> {
    pub generate: &'a Generate,
    pub search: Option<&'a Searching>,
    pub cancel: &'a AtomicBool,
    pub silence: Option<Duration>,
    /// How many silent requests may still be asked again.
    pub retries: Cell<u32>,
    /// A reply may be a request to read (`agent::protocol`), which never
    /// shows as a draft.
    pub reads: bool,
}

impl Asker<'_> {
    /// One streamed reply to `prompt`, its draft reaching the pane; with
    /// `web`, through the searching call. A request silent past
    /// [`FIRST_EVENT`] is stopped and asked once more while a retry is left.
    pub(super) fn ask(&self, prompt: &prompt::Prompt, web: bool) -> Result<String> {
        let search = self.search.filter(|_| web);
        let ask = |stop: &AtomicBool, heard: &Arc<AtomicBool>| {
            let streamed: Streamed = {
                let (heard, drafts) = (heard.clone(), drafts(self.reads));
                Arc::new(move |chars, text| {
                    heard.store(true, Ordering::Release);
                    drafts(chars, text);
                })
            };
            match search {
                Some(search) => {
                    let seen = |item: &Value, done: bool| {
                        heard.store(true, Ordering::Release);
                        seen(item, done);
                    };
                    search(
                        &prompt.instructions,
                        &prompt.input,
                        stop,
                        Some(streamed),
                        &seen,
                    )
                }
                None => (self.generate)(&prompt.instructions, &prompt.input, stop, Some(streamed)),
            }
        };
        let unheard = || Arc::new(AtomicBool::new(false));
        match self.silence.filter(|_| self.retries.get() > 0) {
            Some(silence) => match watched(self.cancel, silence, ask) {
                Some(result) => result,
                None => {
                    tracing::warn!(
                        "Assistant's model said nothing in {} s; asking again",
                        silence.as_secs()
                    );
                    self.retries.set(self.retries.get() - 1);
                    retrying();
                    ask(self.cancel, &unheard())
                }
            },
            None => ask(self.cancel, &unheard()),
        }
    }
}

/// What a turn's thread made: the reply settled, and what it read.
pub(super) struct Answered {
    pub result: Result<Settled>,
    /// The recordings excerpts the model read, which its `[n]` cite.
    pub found: Option<Found>,
    /// The guide articles it read, by id.
    pub articles: Vec<&'static str>,
    /// What the model was told of this install (`facts::text::knew`).
    pub knew: String,
    /// What an agent turn read, in order; empty for the router's.
    pub trace: Vec<Did>,
}

/// One request, with what the router chose for it to read: what a search
/// of the recordings found, the release notes, or the guide alone.
fn one_shot(question: &Question, room: Room, asker: &Asker, local: bool) -> Answered {
    let earlier: Vec<prompt::Turn> = question
        .earlier
        .iter()
        .map(|earlier| prompt::Turn {
            question: &earlier.question,
            answer: &earlier.answer,
            changes: &earlier.changes,
        })
        .collect();
    let found = match &question.recall {
        Recall::Search(search) => read(search, room),
        _ => None,
    };
    let notes = match &question.recall {
        Recall::Releases(asked) => Some(releases::select(asked, releases::Running::here())),
        _ => None,
    };
    let reach = match (&question.recall, &found, &notes) {
        (Recall::Declined, ..) => Reach::Declined,
        (Recall::Search(search), Some(found), _) => Reach::Recordings(found, search.now_ms),
        (_, _, Some(notes)) => Reach::Releases(notes),
        _ => Reach::Guide,
    };
    let mut prompt = prompt::build(
        &question.question,
        &earlier,
        room,
        reach,
        question.changes.as_ref(),
        &question.setup,
    );
    let web = match asker
        .search
        .and(offline(question, found.as_ref(), prompt.vocabulary))
    {
        None => asker.search.is_some(),
        Some(why) => {
            if why == Offline::Earlier {
                held_back();
            }
            false
        }
    };
    if web {
        prompt.instructions.push_str(super::web::RULES);
    }
    #[cfg(test)]
    super::eval::sent(&prompt, web);
    let result = asker.ask(&prompt, web).map(|reply| {
        let again = (!local).then_some(asking_again as fn());
        settle(
            question,
            &prompt,
            reply,
            asker.generate,
            asker.cancel,
            again,
        )
    });
    Answered {
        result,
        found,
        articles: prompt.articles,
        knew: prompt.knew,
        trace: Vec::new(),
    }
}

/// Web is on for the conversation, and this answer stays off it.
pub(super) fn held_back() {
    let mut state = state();
    if let Some(answering) = &mut state.answering {
        answering.web.held_back = true;
    }
    changed(state);
}

/// Searches the recordings, each step reaching the pane as it lands. A
/// question that only may be about them, and matched nothing, is a guide
/// question after all.
fn read(search: &Search, room: Room) -> Option<Found> {
    let budget = match room {
        Room::Small => Budget::SMALL,
        Room::Large => Budget::LARGE,
    };
    let found = recordings::search(
        &search.allowed,
        &search.sources,
        &search.intent,
        &search.vocabulary,
        budget,
        search.now_ms,
        &mut |step| {
            let mut state = state();
            if let Some(answering) = &mut state.answering {
                answering.steps.push(step.clone());
            }
            changed(state);
        },
    );
    let mut state = state();
    if let Some(answering) = &mut state.answering {
        answering.searching = false;
        if !search.intent.sure && found.excerpts.is_empty() {
            answering.steps.clear();
        }
    }
    changed(state);
    (search.intent.sure || !found.excerpts.is_empty()).then_some(found)
}

/// One of Codex's web searches, as a step the pane shows.
/// Runs `ask` with a stop of its own, which `cancel` sets, and so does
/// `silence` passing before `ask` hears anything. None when it was the
/// silence that stopped it.
fn watched<T>(
    cancel: &AtomicBool,
    silence: Duration,
    ask: impl FnOnce(&AtomicBool, &Arc<AtomicBool>) -> Result<T>,
) -> Option<Result<T>> {
    let stop = AtomicBool::new(false);
    let heard = Arc::new(AtomicBool::new(false));
    let done = AtomicBool::new(false);
    let silent = AtomicBool::new(false);
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            let started = Instant::now();
            while !done.load(Ordering::Acquire) {
                if cancel.load(Ordering::Acquire) {
                    stop.store(true, Ordering::Release);
                } else if !heard.load(Ordering::Acquire)
                    && !silent.load(Ordering::Acquire)
                    && started.elapsed() >= silence
                {
                    silent.store(true, Ordering::Release);
                    stop.store(true, Ordering::Release);
                }
                std::thread::sleep(WATCH_EVERY);
            }
        });
        let result = ask(&stop, &heard);
        done.store(true, Ordering::Release);
        result
    });
    let stalled = silent.load(Ordering::Acquire) && !cancel.load(Ordering::Acquire);
    (!stalled).then_some(result)
}

/// The steps say it is asking again.
fn retrying() {
    let mut state = state();
    if let Some(answering) = &mut state.answering {
        answering.retrying = true;
        answering.web = Web::default();
    }
    changed(state);
}

fn seen(item: &Value, done: bool) {
    let mut state = state();
    if let Some(answering) = &mut state.answering {
        answering.web.saw(item, done);
    }
    changed(state);
}

/// Changes the answering state, if a question is being answered, and
/// wakes the window.
pub(super) fn update(change: impl FnOnce(&mut Answering)) {
    let mut state = state();
    if let Some(answering) = &mut state.answering {
        change(answering);
    }
    changed(state);
}

/// Shows "Preparing a change" while a remote model is asked once more.
pub(super) fn asking_again() {
    let mut state = state();
    if let Some(answering) = &mut state.answering {
        answering.cards_coming = true;
    }
    changed(state);
}

/// What of a reply still streaming goes into the draft. Where a reply
/// may ask to read (`reads`), one that does shows nothing, however it is
/// fenced, and nothing of a call shows before its line is whole.
pub(super) fn showing(reads: bool, text: &str) -> &str {
    if !reads {
        text
    } else if agent::protocol::reads(text).is_some() {
        ""
    } else {
        agent::protocol::shown(text)
    }
}

/// The streamed reply into the draft, at most once each [`DRAFT_EVERY`].
fn drafts(reads: bool) -> Streamed {
    let last: Mutex<Option<Instant>> = Mutex::new(None);
    Arc::new(move |_, text| {
        let text = showing(reads, text);
        let now = Instant::now();
        {
            let mut last = last.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if last.is_some_and(|at| now - at < DRAFT_EVERY) {
                return;
            }
            *last = Some(now);
        }
        let mut state = state();
        if let Some(answering) = &mut state.answering {
            answering.draft = prompt::draft(text).trim_end().to_owned();
            answering.cards_coming = prompt::cards_coming(text);
        }
        changed(state);
    })
}

fn finish(
    question: &Question,
    model: &str,
    started: Instant,
    answered: Answered,
    cancel: &AtomicBool,
) {
    let Answered {
        result,
        found,
        articles,
        knew,
        trace,
    } = answered;
    let found = found.as_ref();
    let mut state = state();
    // The clock leaves out the time an agent turn waited on the person.
    let (draft, web, started) = state
        .answering
        .take()
        .map(|answering| (answering.draft, answering.web, answering.started))
        .unwrap_or_else(|| (String::new(), Web::default(), started));
    state.cancel = None;
    let mut turn = Turn {
        question: question.question.clone(),
        model: model.to_owned(),
        seconds: started.elapsed().as_secs_f32(),
        created_ms: now_ms(),
        steps: found.map(|found| found.steps.clone()).unwrap_or_default(),
        trace,
        knew: knew.clone(),
        ..Turn::default()
    };
    // What the gate dropped is part of what the turn keeps; the log gets
    // it without the terms, which may be the person's own words.
    let mut logged = knew;
    if let Ok(settled) = &result {
        turn.knew.push_str(&super::offer::record(&settled.dropped));
        let settings: Vec<_> = settled
            .dropped
            .iter()
            .filter(|dropped| !dropped.term())
            .cloned()
            .collect();
        logged.push_str(&super::offer::record(&settings));
    }
    super::facts::text::log(question.conversation, &logged);
    let read_ok = result.is_ok();
    match result {
        Ok(settled) => {
            let parsed = changes::parse::parse(&settled.reply);
            let (body, sources) = prompt::split_sources(&parsed.body);
            turn.cards = settled.cards;
            turn.refused = settled.refused;
            turn.asks = settled.asks;
            turn.answer = body.to_owned();
            turn.sources = sources.into_iter().map(str::to_owned).collect();
            // An answer from the recordings that drew on the guide too says
            // so in its steps.
            let articles: Vec<String> = turn
                .sources
                .iter()
                .filter(|id| super::guide::article(id).is_some())
                .cloned()
                .collect();
            if !turn.steps.is_empty() && !articles.is_empty() {
                turn.steps.push(Step::Guide { articles });
            }
            // The lines cited are kept with the places; the answer shows
            // their excerpts' `[n]`.
            if let Some(found) = found {
                turn.refs = found.cited(body);
                turn.answer = recordings::cite::plain(body);
            }
        }
        Err(_) if cancel.load(Ordering::Acquire) => {
            turn.answer = draft;
            turn.stopped = true;
        }
        Err(error) => {
            tracing::warn!("Assistant's model failed: {error:#}");
            turn.answer = draft;
            turn.error = Some(format!("{error:#}"));
        }
    }
    // What the setup makes of the reply: a round of follow-ups, a plan,
    // a returning setup's opening.
    setup::flow::replied(
        &question.setup,
        &mut turn,
        read_ok,
        question.changes.as_ref(),
    );
    // The guide it read first, and the pages its answer cites.
    if !web.is_empty() {
        turn.web = web;
        turn.web.guide = articles.iter().map(|id| (*id).to_owned()).collect();
        turn.web.cite(&turn.answer);
    }
    state.finished.push((question.conversation, turn));
    changed(state);
}

/// Stops the question being answered, keeping what came.
pub fn stop() {
    if let Some(cancel) = &state().cancel {
        cancel.store(true, Ordering::Release);
    }
}

/// The question being answered, if one is.
pub fn answering() -> Option<Answering> {
    state().answering.clone()
}

/// Turns finished since the last call, with their conversations.
pub fn take_finished() -> Vec<(u32, Turn)> {
    std::mem::take(&mut state().finished)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;

    /// The worker is one per process; its tests take turns, the scenarios
    /// among them.
    use crate::assistant::eval::SERIAL;

    fn question(text: &str) -> Question {
        Question {
            conversation: 7,
            question: text.into(),
            earlier: vec![Earlier {
                question: "before".into(),
                answer: "earlier answer".into(),
                changes: String::new(),
                recordings: false,
            }],
            recall: Recall::Guide,
            changes: None,
            setup: Stage::None,
            web: false,
            access: Access::Closed,
            now_ms: 0,
        }
    }

    fn with_changes(text: &str) -> Question {
        let mut settings = crate::app_settings::AppSettings::default();
        settings.labs.filler_words = false;
        settings.labs.spoken_formatting = false;
        Question {
            changes: Some(changes::Context {
                settings,
                state: crate::assistant::facts::State::default(),
            }),
            ..question(text)
        }
    }

    const REFUSED: &str = "Turn it on.\n```hex-change\n{\"key\":\"labs.filler_words\",\"value\":true}\n{\"key\":\"labs.teleport\",\"value\":true}\n```\nSources: labs/filler-words";

    #[test]
    fn a_reply_with_changes_becomes_cards_and_refusals() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generate: Generate = Box::new(|_, _, _, _| Ok(REFUSED.into()));
        assert!(start(
            with_changes("q"),
            Room::Small,
            "Local".into(),
            generate,
            true
        ));
        let (_, turn) = finished();
        assert_eq!(turn.answer, "Turn it on.");
        assert_eq!(turn.sources, ["labs/filler-words"]);
        assert_eq!(turn.cards.len(), 1);
        // A built-in model isn't asked again.
        assert_eq!(turn.refused.len(), 1);
        assert_eq!(turn.refused[0].why, changes::Why::Unknown);
    }

    /// A question about one's shortcuts gets the shortcuts card, whatever
    /// the model wrote, and the model is told it comes; a key the model
    /// named joins it.
    #[test]
    fn a_question_about_shortcuts_gets_the_shortcuts_card() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let told = Arc::new(Mutex::new(false));
        let seen = told.clone();
        let generate: Generate = Box::new(move |instructions, _, _, _| {
            *seen.lock().unwrap() = instructions.contains("The app shows a card under your answer");
            Ok("Here they are.\n```hex-change\n{\"key\":\"paste_last_hotkey\",\"value\":\"control+shift+v\"}\n```".into())
        });
        assert!(start(
            with_changes("Help me set up my shortcuts"),
            Room::Small,
            "Local".into(),
            generate,
            true
        ));
        let (_, turn) = finished();
        assert!(*told.lock().unwrap(), "the model wasn't told of the card");
        assert_eq!(turn.cards.len(), 1, "{:?}", turn.cards);
        let card = &turn.cards[0];
        assert_eq!(card.kind, changes::CardKind::Shortcuts);
        let paste = card
            .items
            .iter()
            .find(|item| item.key == "paste_last_hotkey")
            .unwrap();
        assert!(paste.included);
        assert_eq!(card.items.iter().filter(|item| item.included).count(), 1);
    }

    /// A remote model gets one silent round to fix refused lines.
    #[test]
    fn a_remote_model_repairs_its_refused_lines_once() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let asked = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = asked.clone();
        let generate: Generate = Box::new(move |_, input, _, streamed| {
            let mut asked = seen.lock().unwrap();
            asked.push(input.to_owned());
            assert_eq!(
                streamed.is_some(),
                asked.len() == 1,
                "the repair doesn't stream"
            );
            Ok(if asked.len() == 1 {
                REFUSED.into()
            } else {
                "```hex-change\n{\"key\":\"labs.spoken_formatting\",\"value\":true}\n```".into()
            })
        });
        assert!(start(
            with_changes("q"),
            Room::Large,
            "GPT".into(),
            generate,
            false
        ));
        let (_, turn) = finished();
        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 2);
        assert!(asked[1].contains("<refused>"));
        let keys: Vec<&str> = turn
            .cards
            .iter()
            .map(|card| card.items[0].key.as_str())
            .collect();
        assert_eq!(keys, ["labs.filler_words", "labs.spoken_formatting"]);
        // Its fix named another key: the refusal stays.
        assert_eq!(turn.refused.len(), 1);
    }

    /// A refused term comes back fixed into the terms card there is.
    #[test]
    fn a_fixed_term_joins_the_terms_card() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let calls = Arc::new(Mutex::new(0));
        let count = calls.clone();
        let long = "k".repeat(80);
        let generate: Generate = Box::new(move |_, input, _, _| {
            let mut calls = count.lock().unwrap();
            *calls += 1;
            Ok(if *calls == 1 {
                format!(
                    "Добавлю.\n\n```hex-vocab\n{{\"term\":\"Kubernetes\"}}\n{{\"term\":\"{long}\"}}\n```\n\nSources: none"
                )
            } else {
                assert!(input.contains("<refused>"));
                "```hex-vocab\n{\"term\":\"kubectl\",\"sounds_like\":[\"кубкатл\"]}\n{\"term\":\"kubernetes\"}\n```".into()
            })
        });
        assert!(start(
            with_changes("q"),
            Room::Large,
            "GPT".into(),
            generate,
            false
        ));
        let (_, turn) = finished();
        assert_eq!(*calls.lock().unwrap(), 2);
        assert_eq!(turn.cards.len(), 1);
        assert_eq!(turn.cards[0].kind, changes::CardKind::Vocabulary);
        let terms: Vec<String> = turn.cards[0]
            .items
            .iter()
            .map(|item| changes::vocab::terms(&item.to)[0].term.clone())
            .collect();
        assert_eq!(terms, ["Kubernetes", "kubectl"]);
        assert!(turn.refused.is_empty(), "its fix replaced it");
    }

    #[test]
    fn a_still_bad_repair_leaves_the_refusal() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let calls = Arc::new(Mutex::new(0));
        let count = calls.clone();
        let generate: Generate = Box::new(move |_, _, _, _| {
            *count.lock().unwrap() += 1;
            Ok(REFUSED.into())
        });
        assert!(start(
            with_changes("q"),
            Room::Large,
            "GPT".into(),
            generate,
            false
        ));
        let (_, turn) = finished();
        assert_eq!(*calls.lock().unwrap(), 2, "asked once more, no further");
        assert_eq!(turn.cards.len(), 1);
        assert_eq!(turn.refused.len(), 1);
    }

    /// The owner's case: notes already written by Codex, and a reply that
    /// offers Codex as the writer. No card; a remote model writes its
    /// reply again, told the value is set, and that reply is the answer.
    #[test]
    fn a_value_already_set_is_no_card_and_its_words_are_rewritten() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let asked = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = asked.clone();
        let generate: Generate = Box::new(move |_, input, _, _| {
            let mut asked = seen.lock().unwrap();
            asked.push(input.to_owned());
            Ok(if asked.len() == 1 {
                "Notes are written on this Mac; switch them to Codex.\n```hex-change\n{\"key\":\"meetings.notes.writer\",\"value\":\"codex\"}\n```\nSources: none".into()
            } else {
                "Codex already writes your notes.\nSources: none".into()
            })
        });
        let mut question = with_changes("can you recommend anything else?");
        if let Some(context) = &mut question.changes {
            context.settings.meetings.notes.writer = Some("codex:gpt-6.1-terra".into());
            context.settings.dictation_processing.server.codex_model = "gpt-6.1-terra".into();
        }
        assert!(start(
            question.clone(),
            Room::Large,
            "GPT".into(),
            generate,
            false
        ));
        let (_, turn) = finished();
        let asked = asked.lock().unwrap().clone();
        assert_eq!(asked.len(), 2);
        assert!(asked[1].contains("<already_set>\nmeetings.notes.writer = \"codex\""));
        assert!(turn.cards.is_empty(), "{:?}", turn.cards);
        assert!(turn.refused.is_empty());
        assert_eq!(turn.answer, "Codex already writes your notes.");

        // A built-in model isn't asked again: the card just goes.
        let generate: Generate = Box::new(|_, _, _, _| {
            Ok("Switch them.\n```hex-change\n{\"key\":\"meetings.notes.writer\",\"value\":\"codex:gpt-6.1-terra\"}\n```\nSources: none".into())
        });
        assert!(start(question, Room::Small, "Qwen".into(), generate, true));
        let (_, turn) = finished();
        assert!(turn.cards.is_empty());
        assert!(turn.refused.is_empty());
    }

    /// What the gate drops is no card, and the turn keeps what and why
    /// under "What Assistant knew". A setting this OS lacks is told to a
    /// remote model, once, so its words offer no card there isn't.
    #[test]
    fn what_the_gate_drops_is_kept_in_the_turns_record() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generate: Generate = Box::new(|_, _, _, _| {
            Ok("Filter the picker, and drop fillers.\n```hex-change\n{\"key\":\"labs.picker_downloaded\",\"value\":true}\n{\"key\":\"labs.filler_words\",\"value\":true}\n```\nSources: none".into())
        });
        let mut question = with_changes("Which models have I downloaded?");
        if let Some(context) = &mut question.changes {
            context.settings.labs.picker_downloaded = false;
            context.settings.labs.filler_words = false;
        }
        assert!(start(
            question.clone(),
            Room::Small,
            "Qwen".into(),
            generate,
            true
        ));
        let (_, turn) = finished();
        let keys: Vec<&str> = turn
            .cards
            .iter()
            .map(|card| card.items[0].key.as_str())
            .collect();
        assert_eq!(keys, ["labs.filler_words"]);
        assert!(turn.refused.is_empty());
        assert!(
            turn.knew.ends_with(
                "Proposed and not offered:\n\
                 labs.picker_downloaded = true: only a way to look something up\n"
            ),
            "{}",
            turn.knew
        );

        let elsewhere = crate::control::settings::kinds()
            .map(|(key, ..)| key)
            .find(|key| {
                key.starts_with("labs.") && crate::control::settings::unavailable(key).is_some()
            });
        let Some(key) = elsewhere else {
            return;
        };
        let asked = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = asked.clone();
        let generate: Generate = Box::new(move |_, input, _, _| {
            let mut asked = seen.lock().unwrap();
            asked.push(input.to_owned());
            Ok(if asked.len() == 1 {
                format!(
                    "Turn it on.\n```hex-change\n{{\"key\":\"{key}\",\"value\":true}}\n```\nSources: none"
                )
            } else {
                "This computer doesn't have it.\nSources: none".into()
            })
        });
        assert!(start(question, Room::Large, "GPT".into(), generate, false));
        let (_, turn) = finished();
        let asked = asked.lock().unwrap().clone();
        assert_eq!(asked.len(), 2);
        assert!(
            asked[1].contains(&format!("<not_here>\n{key} = true : not available here")),
            "{}",
            asked[1]
        );
        assert!(!asked[1].contains("<already_set>"));
        assert!(turn.cards.is_empty() && turn.refused.is_empty());
        assert_eq!(turn.answer, "This computer doesn't have it.");
        assert!(
            turn.knew
                .contains(&format!("{key} = true: not available here"))
        );
    }

    fn setup(text: &str, stage: Stage) -> Question {
        Question {
            setup: stage,
            ..with_changes(text)
        }
    }

    fn answers() -> Stage {
        Stage::Plan {
            answers: vec![
                setup::Answer {
                    id: "use".into(),
                    chosen: vec!["dictation_code".into(), "meetings".into()],
                    ..Default::default()
                },
                setup::Answer {
                    id: "language".into(),
                    chosen: vec!["ru".into()],
                    ..Default::default()
                },
                setup::Answer {
                    id: "cleanup".into(),
                    chosen: vec!["codex".into()],
                    ..Default::default()
                },
            ],
            line: "use: dictation_code, meetings; language: ru; cleanup: codex".into(),
            since: String::new(),
            terms: vec![setup::since::Candidate {
                term: "Supabase".into(),
                sounds_like: vec!["супабейс".into()],
                sure: true,
            }],
            about: String::new(),
        }
    }

    /// A returning setup's opening: the step of what the app read shows
    /// while the model writes, then its suggestions, and round one folded
    /// with the terms after them, even when the model fails.
    #[test]
    fn a_returning_setups_look_ends_in_round_one() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let look = || Stage::Look {
            facts: "dictations kept: 972, 972 in the last 30 days".into(),
            shown: "- 972 dictations in the last 30 days".into(),
            asks: setup::questions("macos"),
            terms: vec![setup::since::Candidate {
                term: "Granola".into(),
                ..Default::default()
            }],
            step: recordings::Step::Looked {
                dictations: 972,
                meetings: 5,
            },
            about: "Пишу код, диктую в Cursor".into(),
        };
        for fails in [false, true] {
            let generate: Generate = Box::new(move |instructions, input, _, _| {
                assert!(instructions.contains("2 to 4 suggestions"));
                assert!(input.contains("<shown>"));
                let steps = answering().unwrap().steps;
                assert_eq!(steps.len(), 1, "the step shows while it writes");
                if fails {
                    return Err(eyre!("signed out"));
                }
                Ok("- Add Codex to Vocabulary.\n```hex-change\n{\"key\":\"labs.files\",\"value\":true}\n```\nSources: none".into())
            });
            assert!(start(
                setup("", look()),
                Room::Large,
                "GPT".into(),
                generate,
                false
            ));
            let (_, turn) = finished();
            assert!(turn.question.is_empty() && turn.answers.is_empty());
            assert_eq!(turn.error.is_some(), fails);
            if !fails {
                assert_eq!(turn.answer, "- Add Codex to Vocabulary.");
            }
            assert!(turn.cards.is_empty(), "suggestions, never cards");
            assert!(turn.folded && turn.asks.len() == 4);
            assert_eq!(turn.terms[0].term, "Granola");
            // The last setup's own words come back with round one.
            assert_eq!(turn.about, "Пишу код, диктую в Cursor");
            assert!(turn.facts.starts_with("dictations kept"));
            assert!(matches!(
                turn.steps[..],
                [recordings::Step::Looked {
                    dictations: 972,
                    ..
                }]
            ));
        }
    }

    /// A typed request's questions: the model's, else the app's own.
    #[test]
    fn a_setup_request_asks_the_models_questions_or_the_apps() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let written = "Давайте настроим.\n```hex-ask\n{\"id\":\"use\",\"q\":\"Для чего?\",\"options\":[{\"id\":\"meetings\",\"label\":\"Созвоны\"},{\"id\":\"files\",\"label\":\"Файлы\"}]}\n{\"id\":\"language\",\"q\":\"Язык?\",\"options\":[{\"id\":\"ru\",\"label\":\"Русский\"},{\"id\":\"en\",\"label\":\"English\"}]}\n{\"id\":\"cleanup\",\"q\":\"Где чистить?\",\"options\":[{\"id\":\"codex\",\"label\":\"Codex\"},{\"id\":\"none\",\"label\":\"Нет\"}]}\n{\"id\":\"notes\",\"q\":\"Конспекты?\",\"options\":[{\"id\":\"codex\",\"label\":\"Codex\"},{\"id\":\"local\",\"label\":\"Здесь\"}]}\n```\nSources: none";
        for (reply, own) in [(written, false), ("Давайте настроим. Sources: none", true)]
        {
            let reply = reply.to_owned();
            let generate: Generate = Box::new(move |instructions, _, _, _| {
                assert!(instructions.contains("```hex-ask"));
                assert!(!instructions.contains("```hex-change"));
                Ok(reply.clone())
            });
            let question = setup("Помоги мне настроить VibeHex под меня", Stage::Ask);
            assert!(start(question, Room::Large, "GPT".into(), generate, false));
            let (_, turn) = finished();
            assert_eq!(turn.asks.len(), 4);
            assert!(turn.asks.iter().all(|ask| ask.own == own), "{own}");
            assert!(turn.answer.starts_with("Давайте настроим."));
            assert!(!turn.answer.contains("hex-ask"));
        }
    }

    /// The answers' plan: the model's lines as one list, else the rules'.
    #[test]
    fn the_answers_become_one_plan_the_models_or_the_rules() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let good = "Вот что я предлагаю.\n```hex-plan\n{\"key\":\"labs.spoken_formatting\",\"value\":true,\"why\":\"пробелы\"}\n```\n```hex-change\n{\"key\":\"labs.filler_words\",\"value\":true}\n```\nSources: none";
        for (reply, keys) in [
            (good, vec!["labs.spoken_formatting", "labs.filler_words"]),
            (
                "Вот что я предлагаю.\n```hex-plan\n{\"key\":\"labs.teleport\",\"value\":true}\nnot json\n```",
                vec!["transcription.model"],
            ),
        ] {
            let reply = reply.to_owned();
            let generate: Generate = Box::new(move |instructions, input, _, _| {
                assert!(instructions.contains("```hex-plan"));
                assert!(input.contains("<answers>\nuse: dictation_code"));
                Ok(reply.clone())
            });
            assert!(start(
                setup("", answers()),
                Room::Small,
                "Local".into(),
                generate,
                true
            ));
            let (_, turn) = finished();
            assert_eq!(turn.cards.len(), 1);
            assert_eq!(turn.cards[0].kind, changes::CardKind::Plan);
            let got: Vec<&str> = turn.cards[0]
                .items
                .iter()
                .map(|item| item.key.as_str())
                .collect();
            for key in &keys {
                assert!(got.contains(key), "{key}: {got:?}");
            }
            assert_eq!(turn.answers.len(), 3);
            // A bad reply still gets the rules' plan; what it got wrong
            // shows beside it.
            if keys == ["transcription.model"] {
                assert_eq!(turn.refused.len(), 2, "{:?}", turn.refused);
                assert!(got.len() >= 3);
            }
        }
    }

    fn finished() -> (u32, Turn) {
        for _ in 0..500 {
            if let Some(done) = take_finished().pop() {
                return done;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("no answer came");
    }

    /// A cloud model silent past the watchdog is stopped and asked once
    /// more, the steps saying so; the second answer is the turn's.
    #[test]
    fn a_silent_model_is_asked_once_more() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        take_finished();
        let asked = Arc::new(Mutex::new(0));
        let seen_retry = Arc::new(AtomicBool::new(false));
        let generate: Generate = {
            let (asked, seen_retry) = (asked.clone(), seen_retry.clone());
            Box::new(move |_, _, stop, streamed| {
                let round = {
                    let mut asked = asked.lock().unwrap();
                    *asked += 1;
                    *asked
                };
                if round == 1 {
                    // Silent until stopped.
                    while !stop.load(Ordering::Acquire) {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    return Err(eyre!("stopped"));
                }
                if answering().is_some_and(|answering| answering.retrying) {
                    seen_retry.store(true, Ordering::Release);
                }
                if let Some(streamed) = streamed {
                    streamed(5, "Hello");
                }
                Ok("Hello".into())
            })
        };
        assert!(start_with(
            question("Why?"),
            Room::Large,
            "GPT".into(),
            (generate, None),
            (false, Some(Duration::from_millis(100))),
            None
        ));
        let (_, turn) = finished();
        assert_eq!(turn.answer, "Hello");
        assert_eq!(*asked.lock().unwrap(), 2);
        assert!(
            seen_retry.load(Ordering::Acquire),
            "the steps said it asked again"
        );
    }

    /// A model that answers, however slowly after its first sign, is never
    /// asked again; one silent twice fails with its own error.
    #[test]
    fn a_model_that_speaks_is_not_asked_again_and_a_second_silence_fails() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        take_finished();
        let generate: Generate = Box::new(|_, _, _, streamed| {
            if let Some(streamed) = streamed {
                streamed(0, "");
            }
            std::thread::sleep(Duration::from_millis(300));
            Ok("Slow but here".into())
        });
        assert!(start_with(
            question("Why?"),
            Room::Large,
            "GPT".into(),
            (generate, None),
            (false, Some(Duration::from_millis(100))),
            None
        ));
        let (_, turn) = finished();
        assert_eq!(turn.answer, "Slow but here");

        let asked = Arc::new(Mutex::new(0));
        let generate: Generate = {
            let asked = asked.clone();
            Box::new(move |_, _, stop, _| {
                *asked.lock().unwrap() += 1;
                let started = Instant::now();
                // The second turn runs to the engine's own deadline.
                while !stop.load(Ordering::Acquire) {
                    if started.elapsed() > Duration::from_millis(300) {
                        return Err(eyre!("Codex didn't answer within 240 seconds"));
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(eyre!("stopped"))
            })
        };
        assert!(start_with(
            question("Why?"),
            Room::Large,
            "GPT".into(),
            (generate, None),
            (false, Some(Duration::from_millis(100))),
            None
        ));
        let (_, turn) = finished();
        assert_eq!(*asked.lock().unwrap(), 2, "asked again once, not more");
        assert!(
            turn.error
                .as_deref()
                .is_some_and(|error| error.contains("didn't answer")),
            "{:?}",
            turn.error
        );
    }

    #[test]
    fn a_reply_becomes_a_turn_with_its_sources() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generate: Generate = Box::new(|instructions, input, _, streamed| {
            assert!(instructions.contains("Sources:"));
            assert!(input.contains("earlier answer") && input.contains("Zoom"));
            streamed.unwrap()(5, "Yes.\nSour");
            Ok("Yes.\n\nSources: labs/meetings".into())
        });
        assert!(start(
            question("Zoom?"),
            Room::Large,
            "GPT".into(),
            generate,
            false
        ));
        let (conversation, turn) = finished();
        assert_eq!(conversation, 7);
        assert_eq!(turn.answer, "Yes.");
        assert_eq!(turn.sources, ["labs/meetings"]);
        assert_eq!((turn.model.as_str(), turn.stopped), ("GPT", false));
        assert!(answering().is_none());
    }

    /// Only Codex searches, only with the conversation's Web on, and never
    /// for guided setup; every other engine asks as before.
    #[test]
    fn only_a_codex_answer_with_web_on_searches() {
        let mode = crate::app_settings::DictationMode::default();
        let server = crate::app_settings::CleanupServerSettings::default();
        let engine = |choice: &str| Engine::resolve(Some(choice), &server, false, &mode);
        let on = Question {
            web: true,
            ..question("How do I start LM Studio's server?")
        };
        assert!(searches(&on, &engine("codex")));
        assert!(searches(&on, &engine("codex:gpt-6.1-sol")));
        for other in ["local:qwen3-4b", "openrouter", "custom", "opencode"] {
            assert!(!searches(&on, &engine(other)), "{other}");
        }
        let off = Question {
            web: false,
            ..on.clone()
        };
        assert!(!searches(&off, &engine("codex")));
        let setup = Question {
            setup: Stage::Ask,
            ..on.clone()
        };
        assert!(!searches(&setup, &engine("codex")));
    }

    /// Which call an answer made with Web on, once it is in: the one that
    /// may search the web, or the plain one.
    fn called_with_web(question: Question) -> (bool, Turn) {
        let searched = Arc::new(AtomicBool::new(false));
        let generate: Generate = Box::new(|instructions, _, _, _| {
            assert!(!instructions.contains(super::super::web::RULES));
            Ok("From the guide.\n\nSources: topics/assistant".into())
        });
        let flag = searched.clone();
        let search: Searching = Box::new(move |_, _, _, _, _| {
            flag.store(true, Ordering::SeqCst);
            Ok("From the web.\n\nSources: topics/assistant".into())
        });
        assert!(start_with(
            Question {
                web: true,
                ..question
            },
            Room::Large,
            "GPT".into(),
            (generate, Some(search)),
            (false, None),
            None
        ));
        let (_, turn) = finished();
        (searched.load(Ordering::SeqCst), turn)
    }

    /// The person's recordings never go online, whatever the prompt says:
    /// a question about them, Web on, makes the call without web search,
    /// and says nothing of it; a guide question with Web on searches.
    #[test]
    fn a_recordings_answer_never_searches_the_web() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assistant/recordings");
        let now_ms =
            crate::meeting::query::parse_time("2026-11-27", 0, false).unwrap() + 12 * 3_600_000;
        let asked = "Summarize my calls with Anna last week";
        let recordings = Question {
            recall: Recall::Search(Box::new(Search {
                allowed: Allowed::from_settings(&crate::app_settings::AssistantSettings {
                    read_recordings: Some(true),
                    ..Default::default()
                })
                .unwrap(),
                sources: Sources::at(&root),
                intent: super::super::intent::read(asked, now_ms).unwrap(),
                vocabulary: Vec::new(),
                now_ms,
            })),
            ..question(asked)
        };
        let (searched, turn) = called_with_web(recordings);
        assert!(!searched, "a recordings question went online");
        assert!(!turn.refs.is_empty() || !turn.steps.is_empty());
        assert!(
            turn.web.is_empty(),
            "nothing to say: it never needed the web"
        );

        let declined = Question {
            recall: Recall::Declined,
            ..question(asked)
        };
        assert!(!called_with_web(declined).0);

        let (searched, turn) = called_with_web(question("How do I start LM Studio's server?"));
        assert!(searched, "a guide question with Web on may search");
        assert!(!turn.web.held_back);
    }

    /// The owner's question about cleanup writing "AI cleanup" for "AI":
    /// the prompt carries their instructions and the Vocabulary entries it
    /// names, and so, with Web on, the call never searches.
    #[test]
    fn a_turn_carrying_vocabulary_stays_off_the_web() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut question = with_changes(
            "можешь проверить, в промпте есть ли какие-то штуки, которые могли привести к этому, для AI cleanup",
        );
        if let Some(context) = &mut question.changes {
            context.settings.vocabulary = vec![crate::dictation::vocabulary::VocabularyEntry {
                term: "AI cleanup".into(),
                sounds_like: vec!["AI".into()],
            }];
        }
        let (searched, _) = called_with_web(question);
        assert!(!searched, "Vocabulary went online");
    }

    /// After an answer from the recordings, the conversation's next answers
    /// stay off the web, since the earlier answer goes with them; a
    /// question that might have needed it says so in its steps.
    #[test]
    fn after_an_answer_from_the_recordings_the_web_stays_off() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut after = question("How do I start LM Studio's server?");
        after.earlier[0].recordings = true;
        let (searched, turn) = called_with_web(after);
        assert!(!searched);
        assert!(turn.web.held_back && !turn.web.went_online());
        let stored: Turn = serde_json::from_str(&serde_json::to_string(&turn).unwrap()).unwrap();
        assert!(stored.web.held_back);
        assert!(!stored.read_recordings());
        let quoted = Turn {
            steps: vec![Step::Meetings {
                searched: 3,
                found: 2,
            }],
            ..Turn::default()
        };
        assert!(quoted.read_recordings());
        let empty = Turn {
            steps: vec![Step::Dictations {
                searched: 5,
                found: 0,
            }],
            ..Turn::default()
        };
        assert!(!empty.read_recordings(), "a search that found nothing");
    }

    /// A searching answer reads the web rules; its searches land as steps
    /// while it streams, and with the turn, which keeps the pages it cites.
    #[test]
    fn a_searching_answer_keeps_its_searches_and_cited_pages() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generate: Generate = Box::new(|instructions, _, _, _| {
            assert!(!instructions.contains(super::super::web::RULES));
            Ok("Repaired.".into())
        });
        let search: Searching = Box::new(|instructions, _, _, streamed, seen| {
            assert!(instructions.ends_with(super::super::web::RULES));
            let item = serde_json::json!({ "id": "ws_1", "query": "LM Studio local server",
                "action": { "type": "search", "query": "LM Studio local server" } });
            seen(&serde_json::json!({ "id": "ws_1", "query": "" }), false);
            assert!(answering().is_some_and(|answering| answering.web.busy()));
            seen(&item, true);
            assert_eq!(
                answering().map(|answering| answering.web.searches),
                Some(vec!["LM Studio local server".to_owned()])
            );
            streamed.unwrap()(5, "Open");
            Ok("Open the Developer tab ([lmstudio.ai](https://lmstudio.ai/docs/app)).\n\nSources: topics/assistant".into())
        });
        assert!(start_with(
            Question {
                web: true,
                ..question("How do I start LM Studio's server?")
            },
            Room::Large,
            "GPT".into(),
            (generate, Some(search)),
            (false, None),
            None
        ));
        let (_, turn) = finished();
        assert_eq!(turn.web.searches, ["LM Studio local server"]);
        assert_eq!(turn.web.sources, ["https://lmstudio.ai/docs/app"]);
        assert!(!turn.web.guide.is_empty(), "the guide it read first");
        assert!(!turn.web.busy());
        assert_eq!(turn.sources, ["topics/assistant"]);
    }

    /// A real answer with the web on, on the Codex signed in here:
    /// `cargo nextest run --run-ignored only searches_the_web_on_the_installed_codex --no-capture`.
    /// Its web search items show at debug level.
    #[test]
    #[ignore = "needs the Codex CLI, signed in; searches the web"]
    fn searches_the_web_on_the_installed_codex() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = tracing_subscriber::fmt()
            .with_env_filter("hex::dictation::cleanup_server::codex=debug")
            .with_writer(std::io::stderr)
            .try_init();
        let mode = crate::app_settings::DictationMode::default();
        let server = crate::app_settings::CleanupServerSettings::default();
        let engine = Engine::resolve(Some("codex"), &server, false, &mode).with_effort("medium");
        let question = Question {
            earlier: Vec::new(),
            web: true,
            ..question("How do I start LM Studio's local server?")
        };
        assert!(ask(question, engine));
        let started = Instant::now();
        let mut live = (0, 0, 0);
        let turn = loop {
            if let Some((_, turn)) = take_finished().pop() {
                break turn;
            }
            if let Some(answering) = answering() {
                live = live.max(answering.web.progress());
            }
            assert!(started.elapsed() < Duration::from_secs(300), "no answer");
            std::thread::sleep(Duration::from_millis(50));
        };
        eprintln!("live web steps (searches, pages, going): {live:?}\n{turn:#?}");
        assert!(turn.error.is_none(), "{:?}", turn.error);
        assert!(turn.web.went_online(), "no web search arrived");
    }

    #[test]
    fn a_stop_keeps_what_came() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generate: Generate = Box::new(|_, _, cancel, streamed| {
            streamed.unwrap()(7, "Partial");
            while !cancel.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(eyre!("stopped"))
        });
        assert!(start(
            question("q"),
            Room::Large,
            "GPT".into(),
            generate,
            false
        ));
        while answering().is_none_or(|answering| answering.draft.is_empty()) {
            std::thread::sleep(Duration::from_millis(5));
        }
        let busy: Generate = Box::new(|_, _, _, _| Ok(String::new()));
        assert!(!start(
            question("again"),
            Room::Large,
            "GPT".into(),
            busy,
            false
        ));
        stop();
        let (_, turn) = finished();
        assert!(turn.stopped);
        assert_eq!(turn.answer, "Partial");
        assert!(turn.error.is_none());
    }

    #[test]
    fn a_failure_is_the_turns_error() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generate: Generate = Box::new(|_, _, _, _| Err(eyre!("not signed in")));
        assert!(start(
            question("q"),
            Room::Small,
            "GPT".into(),
            generate,
            false
        ));
        let (_, turn) = finished();
        assert_eq!(turn.error.as_deref(), Some("not signed in"));
        assert!(!turn.stopped && turn.answer.is_empty());
    }
}
