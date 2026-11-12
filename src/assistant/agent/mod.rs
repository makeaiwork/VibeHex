//! Assistant as an agent: on a remote engine the model itself asks the app
//! for what to read, a step at a time, instead of the app guessing from
//! the question's words what to put in the prompt.
//!
//! - [`protocol`]: the fenced `hex-read` block a model asks with, and the
//!   `<read>` block its results come back in.
//! - [`table`]: what it may ask for, all of it read-only; [`tools`]: the
//!   reading, within a turn's [`Limits`]; [`access`]: the person's leave
//!   to read their recordings, asked for mid-answer when they haven't said.
//! - [`prompt`]: what the model is told: a guide question's first request,
//!   the rules for asking, and what goes under each step's results.
//! - [`trace`]: what a turn read, as the pane lists it; [`live`]: the same
//!   while it runs, and the consent card a turn parks on.
//!
//! [`run`] is the loop: ask, run what the reply asks to read, ask again
//! with the results under the question, until a reply is an answer or the
//! turn's [`Limits`] say it must be. The answer then goes through the same
//! `settle` every reply does (change blocks, the gate, the repair round).
//! Built-in models, guided setup, the preset, and a meeting's or a file's
//! Requests never come here: they stay one request, read by the router.
//!
//! Whether a step may search the web is decided in one place, [`net`],
//! before each request: never once the turn holds anything of the
//! person's own, and the turn's trace keeps the rest of its conversation
//! off the web too (`Turn::read_recordings`).

pub mod access;
pub mod live;
pub mod prompt;
pub mod protocol;
pub mod table;
pub mod tools;
pub mod trace;
pub mod words;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use color_eyre::eyre::eyre;

use self::access::Access;
use self::tools::Desk;
use self::trace::{Did, Doing};
use super::intent::cleanup;
use super::recordings::Budget;
use super::settle::settle;
use super::worker::{self, Answered, Asker, Question, Recall};
use super::{facts, personal, web};
use crate::app_settings::AssistantSettings;
use crate::files::rewrite::Engine;

/// How much one turn may ask and read.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Requests to the model; the last is told to answer.
    pub requests: usize,
    /// Calls the app runs of one reply's block.
    pub step_calls: usize,
    /// Calls in all.
    pub turn_calls: usize,
    /// Characters of one result.
    pub result_chars: usize,
    /// Characters of every result together.
    pub turn_chars: usize,
    /// One search of the recordings.
    pub search: Budget,
    /// The model's own time, after which the next request is the last.
    pub model_time: Duration,
}

impl Limits {
    /// A cloud model with a long window.
    pub const REMOTE: Self = Self {
        requests: 6,
        step_calls: 4,
        turn_calls: 12,
        result_chars: 9_000,
        turn_chars: 60_000,
        search: Budget::LARGE,
        model_time: Duration::from_secs(300),
    };
    /// The person's own server, whose window may be short.
    pub const OWN_SERVER: Self = Self {
        turn_chars: 16_000,
        search: Budget::SMALL,
        ..Self::REMOTE
    };
}

/// The engines agent mode is on for while `assistant.agent` is empty: the
/// ones a live run of the scenarios passed on (docs/features/assistant.md
/// › Agent mode). OpenRouter, a person's own server, and OpenCode have had
/// no such run, and answer through the router until one names them here.
pub const VERIFIED: &[&str] = &["codex"];

/// How `engine` is named in `assistant.agent`; none for a built-in model,
/// which never answers as an agent.
fn provider(engine: &Engine) -> Option<&'static str> {
    if engine.is_local() {
        None
    } else if matches!(engine, Engine::OpenCode(_)) {
        Some("opencode")
    } else if engine.searches_web() {
        Some("codex")
    } else if engine.streams_from_the_cloud() {
        Some("openrouter")
    } else {
        Some("custom")
    }
}

/// Whether `provider` answers as an agent: `HEX_ASSISTANT_AGENT` (`1` or
/// `0`) when set, else `assistant.agent`: empty for [`VERIFIED`], `off`,
/// `all`, or a list of providers.
fn on(provider: &str, setting: &str, forced: Option<&str>) -> bool {
    match forced.map(str::trim) {
        Some("1") => return true,
        Some("0") => return false,
        _ => {}
    }
    match setting.trim().to_lowercase().as_str() {
        "" => VERIFIED.contains(&provider),
        "off" => false,
        "all" => true,
        list => list.split(',').any(|name| name.trim() == provider),
    }
}

/// What a turn on `engine` may ask and read as an agent; none when it
/// answers in one request, as the router has it.
pub fn limits(engine: &Engine, settings: Option<&AssistantSettings>) -> Option<Limits> {
    let provider = provider(engine)?;
    let forced = std::env::var("HEX_ASSISTANT_AGENT").ok();
    let setting = settings.map_or("", |settings| settings.agent.as_str());
    on(provider, setting, forced.as_deref()).then_some(match provider {
        "custom" => Limits::OWN_SERVER,
        _ => Limits::REMOTE,
    })
}

/// Whether a request may search the web.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Net {
    /// This turn never could: Web is off, or the engine has none.
    None,
    On,
    /// It could, and what the turn or its conversation holds keeps it off.
    Off,
}

/// Whether the app's own reading says the turn is about what is the
/// person's own from its first request on, as the router's prompt would
/// hold it: a question that is or may be about their recordings (a word
/// dictation wrote as another too: the router searches History for it),
/// or one that names entries of their Vocabulary.
fn own(question: &Question) -> bool {
    if matches!(question.recall, Recall::Search(_)) {
        return true;
    }
    let Some(context) = question.changes.as_ref().filter(|context| context.send()) else {
        return false;
    };
    if !cleanup::wants(&question.question).vocabulary {
        return false;
    }
    let mut texts = vec![question.question.as_str()];
    for turn in question.earlier.iter().rev().take(2) {
        texts.extend([turn.question.as_str(), turn.answer.as_str()]);
    }
    personal::vocabulary(&context.settings, &texts).1 > 0
}

/// The one place that decides, before each request, whether it may search
/// the web: never once the turn has read anything of the person's own
/// (recordings, a list of them, what their use shows, their cleanup
/// instructions, Vocabulary entries), never after an earlier answer that
/// did, never for a question about recordings they said no to, and never
/// for a question the app itself reads as about their own ([`own`]): the
/// first request holds nothing of theirs, but the web has nothing for
/// such a question, and its words are not for a search engine.
pub(super) fn net(searches: bool, question: &Question, desk: &Desk) -> Net {
    if !searches {
        Net::None
    } else if desk.personal()
        || matches!(question.recall, Recall::Declined)
        || question.earlier.iter().any(|earlier| earlier.recordings)
        || own(question)
    {
        Net::Off
    } else {
        Net::On
    }
}

/// Whether a turn whose web is off says so (`Web::held_back`), as the
/// router does: when the web was on earlier in the turn and what it read
/// since turned it off, or when only an earlier answer keeps it off. A
/// question about their own never had the web to lose, and nothing is said.
fn says_held_back(question: &Question, was_on: bool) -> bool {
    was_on
        || (!own(question)
            && !matches!(question.recall, Recall::Declined)
            && question.earlier.iter().any(|earlier| earlier.recordings))
}

/// Answers `question` as an agent, within `limits`.
pub(super) fn run(question: &Question, asker: &Asker, limits: Limits) -> Answered {
    let conversation = question.conversation;
    // A no, before the turn or in an earlier one, closes the recordings
    // for the conversation; the setting turned on opens them all the same.
    let access = match (&question.recall, question.access.clone()) {
        (Recall::Declined, Access::Ask { .. }) => {
            live::decline(conversation);
            Access::Closed
        }
        (_, Access::Ask { .. }) if live::declined(conversation) => Access::Closed,
        (_, access) => access,
    };
    let mut desk = Desk::new(question.changes.as_ref(), access, limits, question.now_ms);
    let first = prompt::first(question, limits);
    desk.laid_out(&first.articles);
    let mut trace = Vec::new();
    let mut keep = |did: Did| {
        live::did(&did);
        trace.push(did);
    };
    keep(Did::Articles {
        ids: first.articles.iter().map(|id| (*id).to_owned()).collect(),
        app: true,
    });
    let pane = live::Pane {
        cancel: asker.cancel,
        conversation,
    };
    let stopped = || asker.cancel.load(Ordering::Acquire);
    let mut results = String::new();
    let mut model_time = Duration::ZERO;
    let mut held_back = false;
    let mut was_on = false;
    let mut request = 0;
    let result = loop {
        request += 1;
        let last = request >= limits.requests || desk.spent() || model_time >= limits.model_time;
        let net = net(asker.search.is_some(), question, &desk);
        if net == Net::Off && !held_back && says_held_back(question, was_on) {
            held_back = true;
            worker::held_back();
        }
        was_on |= net == Net::On;
        let prompt = crate::assistant::prompt::Prompt {
            instructions: format!(
                "{}{}",
                first.instructions,
                match net {
                    Net::None => "",
                    Net::On => web::RULES,
                    Net::Off => prompt::WEB_OFF,
                }
            ),
            input: format!(
                "{}{results}{}",
                first.input,
                prompt::tail(&desk, last, request > 1)
            ),
            articles: desk.articles().to_vec(),
            vocabulary: false,
            knew: String::new(),
        };
        #[cfg(test)]
        super::eval::sent(&prompt, net == Net::On);
        live::doing(Some(Doing::Thinking));
        let asked = Instant::now();
        let reply = match asker.ask(&prompt, net == Net::On) {
            Ok(reply) => reply,
            Err(error) => break Err(error),
        };
        model_time += asked.elapsed();
        let reads = match last {
            // The last request's reply is the answer, whatever it asks.
            true => None,
            false => protocol::reads(&reply),
        };
        let Some(reads) = reads else {
            live::doing(None);
            // Never a call shown as words: the last reply's block, and a
            // call line an answer carries with no fence.
            let answer = protocol::without(&reply);
            if answer.trim().is_empty() {
                break Err(eyre!("the model kept asking to read and never answered"));
            }
            let again = Some(worker::asking_again as fn());
            break Ok(settle(
                question,
                &prompt,
                answer,
                asker.generate,
                asker.cancel,
                again,
            ));
        };
        live::asked_to_read();
        let mut step = Vec::new();
        for (index, call) in reads.calls.iter().enumerate() {
            if stopped() {
                break;
            }
            #[cfg(test)]
            super::eval::called(&call.tool);
            let text = if index < limits.step_calls {
                let tool = table::TOOLS.iter().find(|tool| tool.name == call.tool);
                live::doing(tool.map(|tool| Doing::Reading(tool.name)));
                let ran = desk.run(call, &pane);
                ran.did.into_iter().for_each(&mut keep);
                ran.text
            } else {
                format!(
                    "Not run: at most {} calls in one block. Ask again if you still need it.",
                    limits.step_calls
                )
            };
            step.push(protocol::Result {
                line: call.line.clone(),
                tool: call.tool.clone(),
                text,
            });
        }
        if stopped() {
            break Err(eyre!("stopped"));
        }
        step.extend(reads.malformed.iter().map(|line| protocol::Result {
            line: line.clone(),
            tool: String::new(),
            text: "Not read: this line isn't one JSON object naming a \"tool\".".to_owned(),
        }));
        results.push_str(&protocol::rendered(request, &step));
    };
    live::doing(None);
    let read = desk.read();
    let knew = question.changes.as_ref().map_or(String::new(), |context| {
        facts::text::knew(
            &context.facts(),
            &facts::text::Sent {
                articles: desk.articles().len(),
                instructions: read.instructions,
                vocabulary: read.vocabulary,
                recordings: desk.shelf().found().excerpts.len(),
            },
        )
    });
    Answered {
        result,
        articles: desk.articles().to_vec(),
        knew,
        found: (!desk.shelf().is_empty()).then(|| desk.into_found()),
        trace,
    }
}

#[cfg(test)]
mod tests;
