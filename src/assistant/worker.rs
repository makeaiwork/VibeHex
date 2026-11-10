//! Assistant's answers run on a thread of their own, never behind a Files
//! job: one question at a time, its reply streaming into a draft the pane
//! reads. A stop keeps what came; a failure becomes the turn's error. The
//! pane takes finished turns into its store.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;

use super::prompt::{self, Room};
use super::store::Turn;
use crate::dictation::cleanup_pieces::Streamed;
use crate::dictation::cleanup_server::engine;
use crate::files::rewrite::Engine;

/// How often a growing draft reaches the pane.
const DRAFT_EVERY: Duration = Duration::from_millis(80);

type Generate =
    Box<dyn Fn(&str, &str, &AtomicBool, Option<Streamed>) -> Result<String> + Send + 'static>;

/// A question for a conversation, with its earlier turns.
#[derive(Clone, Debug)]
pub struct Question {
    pub conversation: u32,
    pub question: String,
    /// Earlier (question, answer) pairs, oldest first.
    pub earlier: Vec<(String, String)>,
}

/// The question being answered and what has come so far.
#[derive(Clone, Debug)]
pub struct Answering {
    pub conversation: u32,
    pub question: String,
    /// The reply so far, without a half-written `Sources:` line.
    pub draft: String,
    pub started: Instant,
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
    let generate: Generate = Box::new(move |instructions, input, cancel, streamed| {
        engine.generate(context, instructions, input, cancel, streamed)
    });
    start(question, room, model, generate, local)
}

fn start(question: Question, room: Room, model: String, generate: Generate, local: bool) -> bool {
    let mut held = state();
    if held.answering.is_some() {
        return false;
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let started = Instant::now();
    held.answering = Some(Answering {
        conversation: question.conversation,
        question: question.question.clone(),
        draft: String::new(),
        started,
    });
    held.cancel = Some(cancel.clone());
    changed(held);
    let spawned = std::thread::Builder::new()
        .name("assistant".into())
        .spawn(move || {
            let earlier: Vec<prompt::Turn> = question
                .earlier
                .iter()
                .map(|(asked, answered)| prompt::Turn {
                    question: asked,
                    answer: answered,
                })
                .collect();
            let prompt = prompt::build(&question.question, &earlier, room);
            let result = generate(&prompt.instructions, &prompt.input, &cancel, Some(drafts()));
            finish(&question, &model, started, result, &cancel);
            if local {
                engine::narrow();
            }
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

/// The streamed reply into the draft, at most once each [`DRAFT_EVERY`].
fn drafts() -> Streamed {
    let last: Mutex<Option<Instant>> = Mutex::new(None);
    Arc::new(move |_, text| {
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
        }
        changed(state);
    })
}

fn finish(
    question: &Question,
    model: &str,
    started: Instant,
    result: Result<String>,
    cancel: &AtomicBool,
) {
    let mut state = state();
    let draft = state
        .answering
        .take()
        .map(|answering| answering.draft)
        .unwrap_or_default();
    state.cancel = None;
    let mut turn = Turn {
        question: question.question.clone(),
        model: model.to_owned(),
        seconds: started.elapsed().as_secs_f32(),
        created_ms: now_ms(),
        ..Turn::default()
    };
    match result {
        Ok(reply) => {
            let (body, sources) = prompt::split_sources(&reply);
            turn.answer = body.to_owned();
            turn.sources = sources.into_iter().map(str::to_owned).collect();
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

    /// The worker is one per process; its tests take turns.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn question(text: &str) -> Question {
        Question {
            conversation: 7,
            question: text.into(),
            earlier: vec![("before".into(), "earlier answer".into())],
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
