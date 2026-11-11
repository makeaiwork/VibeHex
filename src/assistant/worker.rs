//! Assistant's answers run on a thread of their own, never behind a Files
//! job: one question at a time, its reply streaming into a draft the pane
//! reads. A stop keeps what came; a failure becomes the turn's error. The
//! pane takes finished turns into its store.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;

use super::changes::{self, Card, CardKind, Refused, Why};
use super::intent::Intent;
use super::prompt::{self, Reach, Room};
use super::recordings::{self, Allowed, Budget, Found, Sources, Step};
use super::releases;
use super::setup::{self, Ask, Stage};
use super::store::Turn;
use crate::dictation::cleanup_pieces::Streamed;
use crate::dictation::cleanup_server::engine;
use crate::dictation::vocabulary::VocabularyEntry;
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
    /// Earlier turns, oldest first.
    pub earlier: Vec<Earlier>,
    /// What it may read besides the guide.
    pub recall: Recall,
    /// The settings it may suggest changes to, if it may.
    pub changes: Option<changes::Context>,
    /// Guided setup's questions or plan, or neither.
    pub setup: Stage,
}

/// An earlier question, its answer, and what became of its changes.
#[derive(Clone, Debug, Default)]
pub struct Earlier {
    pub question: String,
    pub answer: String,
    pub changes: String,
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
    /// A change block has started: the feed shows "Preparing a change…".
    pub cards_coming: bool,
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
        // Setup's answers show as chips, never as a question.
        question: match question.setup {
            Stage::Plan { .. } | Stage::Follow { .. } => String::new(),
            _ => question.question.clone(),
        },
        draft: String::new(),
        started,
        steps: Vec::new(),
        searching: matches!(question.recall, Recall::Search(_)),
        cards_coming: false,
    });
    held.cancel = Some(cancel.clone());
    changed(held);
    let spawned = std::thread::Builder::new()
        .name("assistant".into())
        .spawn(move || {
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
            let prompt = prompt::build(
                &question.question,
                &earlier,
                room,
                reach,
                question.changes.as_ref(),
                &question.setup,
            );
            let result = generate(&prompt.instructions, &prompt.input, &cancel, Some(drafts()))
                .map(|reply| settle(&question, &prompt, reply, &generate, &cancel, !local));
            finish(&question, &model, started, result, &cancel, found.as_ref());
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

/// A reply with its change lines read and checked.
struct Settled {
    reply: String,
    cards: Vec<Card>,
    refused: Vec<Refused>,
    /// Setup's questions, when it asked them.
    asks: Vec<Ask>,
}

/// Reads `reply`'s change lines against the question's settings. A remote
/// model whose lines were refused is asked once more to fix them; a local
/// one isn't, and its refusals stay as they are.
fn settle(
    question: &Question,
    prompt: &prompt::Prompt,
    reply: String,
    generate: &Generate,
    cancel: &AtomicBool,
    remote: bool,
) -> Settled {
    let mut parsed = changes::parse::parse(&reply);
    let plan = matches!(question.setup, Stage::Plan { .. });
    let os = question
        .changes
        .as_ref()
        .map_or(std::env::consts::OS, |context| context.state.os);
    let asks = match (&question.setup, &question.changes) {
        (Stage::Ask, context) => {
            let mut asks = setup::asks(&parsed.asks, os);
            if let Some(context) = context {
                setup::preselect(&mut asks, &context.settings, &context.state);
            }
            asks
        }
        // Follow-up questions only: any change lines are dropped.
        (Stage::Follow { candidates, .. }, Some(context)) => {
            let asks = setup::follow::follow_ups(&parsed.asks, candidates, &context.settings);
            return Settled {
                reply,
                cards: Vec::new(),
                refused: Vec::new(),
                asks,
            };
        }
        (Stage::Follow { candidates, .. }, None) => candidates.clone(),
        _ => Vec::new(),
    };
    let Some(context) = &question.changes else {
        return Settled {
            reply,
            cards: Vec::new(),
            refused: Vec::new(),
            asks,
        };
    };
    // Setup's answer is one list, whichever fence each line came in.
    if plan {
        plan_lines(&mut parsed.proposals);
    }
    let rules = crate::rune_ui::shortcut_rules();
    let (mut cards, mut refused) = changes::resolve(&parsed.proposals, &context.settings, &rules);
    refused.extend(malformed(&parsed.malformed));
    let fixable: Vec<Refused> = refused
        .iter()
        .filter(|refused| refused.why != Why::Never)
        .cloned()
        .collect();
    if remote && !fixable.is_empty() && !cancel.load(Ordering::Acquire) {
        {
            let mut state = state();
            if let Some(answering) = &mut state.answering {
                answering.cards_coming = true;
            }
            changed(state);
        }
        let again = prompt::repair(prompt, &reply, &fixable);
        if let Ok(fixed) = generate(&again.instructions, &again.input, cancel, None) {
            let mut parsed = changes::parse::parse(&fixed);
            if plan {
                plan_lines(&mut parsed.proposals);
            }
            let (more, _) = changes::resolve(&parsed.proposals, &context.settings, &rules);
            let fixed: Vec<String> = more
                .iter()
                .flat_map(|card| card.items.iter().map(|item| item.key.clone()))
                .collect();
            for card in more {
                // Fixed terms join the terms card there is.
                if card.kind == CardKind::Vocabulary
                    && let Some(terms) = cards
                        .iter_mut()
                        .find(|known| known.kind == CardKind::Vocabulary)
                {
                    changes::vocab::merge(terms, card);
                } else if !cards.iter().any(|known| same_keys(known, &card)) {
                    cards.push(card);
                }
            }
            // A refusal its fix replaced goes; the rest stay.
            if !fixed.is_empty() {
                refused.retain(|refused| {
                    refused.why == Why::Never
                        || refused.key.as_ref().is_some_and(|key| !fixed.contains(key))
                });
            }
        }
    }
    if plan {
        // The app's rules decide; the model's lines join them.
        cards = setup::plan::plan(
            cards,
            question.setup.answers(),
            &context.settings,
            &context.state,
        )
        .into_iter()
        .collect();
    }
    Settled {
        reply,
        cards,
        refused,
        asks,
    }
}

/// Every line a plan's reply holds, as a line of its one list.
fn plan_lines(proposals: &mut [changes::parse::Proposal]) {
    for proposal in proposals {
        proposal.plan = true;
    }
}

fn same_keys(one: &Card, other: &Card) -> bool {
    one.items
        .iter()
        .map(|item| &item.key)
        .eq(other.items.iter().map(|item| &item.key))
}

fn malformed(lines: &[String]) -> impl Iterator<Item = Refused> + '_ {
    lines.iter().map(|line| Refused {
        line: line.clone(),
        key: None,
        why: Why::Malformed,
    })
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
            answering.cards_coming = prompt::cards_coming(text);
        }
        changed(state);
    })
}

fn finish(
    question: &Question,
    model: &str,
    started: Instant,
    result: Result<Settled>,
    cancel: &AtomicBool,
    found: Option<&Found>,
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
        steps: found.map(|found| found.steps.clone()).unwrap_or_default(),
        answers: question.setup.answers().to_vec(),
        follow: matches!(question.setup, Stage::Follow { .. }),
        ..Turn::default()
    };
    match result {
        Ok(settled) => {
            let parsed = changes::parse::parse(&settled.reply);
            let (body, sources) = prompt::split_sources(&parsed.body);
            turn.cards = settled.cards;
            turn.refused = settled.refused;
            turn.asks = settled.asks;
            if turn.follow {
                // One round at a time; the rest wait.
                let rest = turn
                    .asks
                    .split_off(turn.asks.len().min(setup::follow::ROUND));
                turn.more = rest;
            }
            if question.setup == Stage::Ask
                && let Some(context) = &question.changes
            {
                turn.now = setup::now::now(&context.settings);
            }
            turn.answer = body.to_owned();
            turn.sources = sources.into_iter().map(str::to_owned).collect();
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
            earlier: vec![Earlier {
                question: "before".into(),
                answer: "earlier answer".into(),
                changes: String::new(),
            }],
            recall: Recall::Guide,
            changes: None,
            setup: Stage::None,
        }
    }

    fn with_changes(text: &str) -> Question {
        let mut settings = crate::app_settings::AppSettings::default();
        settings.labs.smart_spacing = false;
        Question {
            changes: Some(changes::Context {
                settings,
                send: true,
                state: changes::catalog::State::default(),
            }),
            ..question(text)
        }
    }

    const REFUSED: &str = "Turn it on.\n```hex-change\n{\"key\":\"labs.smart_spacing\",\"value\":true}\n{\"key\":\"labs.teleport\",\"value\":true}\n```\nSources: labs/smart-spacing";

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
        assert_eq!(turn.sources, ["labs/smart-spacing"]);
        assert_eq!(turn.cards.len(), 1);
        // A built-in model isn't asked again.
        assert_eq!(turn.refused.len(), 1);
        assert_eq!(turn.refused[0].why, Why::Unknown);
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
                "```hex-change\n{\"key\":\"labs.files\",\"value\":true}\n```".into()
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
        assert_eq!(keys, ["labs.smart_spacing", "labs.files"]);
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
        assert_eq!(turn.cards[0].kind, CardKind::Vocabulary);
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
