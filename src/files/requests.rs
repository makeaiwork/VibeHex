//! Files' request sessions: a request on a version starts a session, a tab
//! in Requests, and every question asked in it later is a follow-up that
//! reads the same text with the conversation so far. A stopped answer keeps
//! what came, marked Stopped.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use color_eyre::eyre::{Result, eyre};

use super::rewrite::{self, DraftKind, Drafts};
use super::store::{self, FollowUp, Version, VersionKind};
use super::worker::{Config, Stage, Task, drafts, source, update_task};

/// A session's first request.
pub struct Asked<'a> {
    pub from: u32,
    pub prompt: &'a str,
    pub name: &'a str,
    /// Overrides Settings' engine and effort for this session.
    pub engine: Option<&'a str>,
    pub effort: Option<&'a str>,
}

/// Runs a session's first request and adds the session to Requests.
pub(super) fn request(
    task: &Task,
    asked: Asked<'_>,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let record = store::load(&task.record)?;
    let input = source(&record, asked.from)?;
    let engine = config.request_engine(asked.engine, asked.effort);
    let model = engine.model_name();
    update_task(task.id, |task| {
        task.stage = Stage::Working;
        task.model = Some(model.clone());
    });
    let started = Instant::now();
    let (kept, drafts) = keeping(task.id);
    let made = rewrite::request(
        &engine,
        asked.prompt,
        &input,
        config.vocabulary_for_requests(),
        cancel,
        &mut |fraction, parts, window_k| {
            update_task(task.id, |task| {
                task.fraction = fraction;
                task.parts = parts;
                task.window_k = window_k;
            });
        },
        &drafts,
    );
    let (made, stopped) = match made {
        Ok(made) => (made, false),
        Err(error) => match stopped_with(cancel, &kept) {
            Some(text) => (
                rewrite::Made {
                    text,
                    window_k: None,
                    parts: None,
                    failed: None,
                },
                true,
            ),
            None => return Err(error),
        },
    };
    let effort = engine.effort().map(str::to_owned);
    store::update(&task.record, |record| {
        let name = record.unique_name(asked.name);
        record.push(Version {
            kind: VersionKind::Request,
            name,
            text: made.text,
            model,
            from: Some(asked.from),
            prompt: Some(asked.prompt.into()),
            window_k: made.window_k,
            parts: made.parts,
            seconds: started.elapsed().as_secs(),
            created_ms: store::now_ms(),
            failed: made.failed,
            stopped,
            engine: asked.engine.map(str::to_owned),
            effort: asked.effort.map(str::to_owned).or(effort),
            ..Version::default()
        });
    })?;
    Ok(())
}

/// Asks `question` in session `session` and adds the answer to it.
pub(super) fn follow_up(
    task: &Task,
    session: u32,
    question: &str,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let record = store::load(&task.record)?;
    let version = record
        .version(session)
        .ok_or_else(|| eyre!("That session is gone."))?;
    let from = version
        .from
        .ok_or_else(|| eyre!("That session has no text to read."))?;
    let input = source(&record, from)?;
    let engine = config.request_engine(version.engine.as_deref(), version.effort.as_deref());
    let model = engine.model_name();
    update_task(task.id, |task| {
        task.stage = Stage::Working;
        task.model = Some(model.clone());
    });
    let first = version.prompt.as_deref().unwrap_or(version.name.as_str());
    let mut earlier = vec![(first, version.text.as_str())];
    earlier.extend(
        version
            .follow_ups
            .iter()
            .map(|asked| (asked.question.as_str(), asked.answer.as_str())),
    );
    let prompt = rewrite::follow_up_prompt(&earlier, question);
    let started = Instant::now();
    let (kept, drafts) = keeping(task.id);
    let made = rewrite::request(
        &engine,
        &prompt,
        &input,
        config.vocabulary_for_requests(),
        cancel,
        &mut |fraction, parts, window_k| {
            update_task(task.id, |task| {
                task.fraction = fraction;
                task.parts = parts;
                task.window_k = window_k;
            });
        },
        &drafts,
    );
    let (answer, stopped) = match made {
        Ok(made) => (made.text, false),
        Err(error) => match stopped_with(cancel, &kept) {
            Some(text) => (text, true),
            None => return Err(error),
        },
    };
    store::update(&task.record, |record| {
        if let Some(version) = record.version_mut(session) {
            version.follow_ups.push(FollowUp {
                question: question.trim().into(),
                answer,
                model,
                seconds: started.elapsed().as_secs(),
                created_ms: store::now_ms(),
                stopped,
            });
        }
    })?;
    Ok(())
}

/// How a question about a meeting reads its transcript.
const MEETING_GUIDE: &str = "The text is a meeting's transcript: each turn names who spoke (You or \
     Them) and when. Answer from it; where it helps, give the time something was said, as \
     (12:40). Say so when the transcript doesn't tell.";

/// Asks `question` in session `session` about the meeting that `task`'s
/// record names, reading `transcript`, and adds the answer to the session.
/// A first question that fails leaves no empty session behind.
pub(super) fn meeting(
    task: &Task,
    session: u32,
    question: &str,
    transcript: &str,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let id = crate::meeting::requests::meeting_of(&task.record)
        .ok_or_else(|| eyre!("That isn't a meeting."))?;
    let answered = meeting_answer(task, id, session, question, transcript, config, cancel);
    if answered.is_err()
        && let Err(error) = crate::meeting::requests::drop_if_empty(id, session)
    {
        tracing::warn!("couldn't tidy the meeting's requests: {error:#}");
    }
    answered
}

fn meeting_answer(
    task: &Task,
    id: &str,
    session: u32,
    question: &str,
    transcript: &str,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let sessions = crate::meeting::requests::load(id);
    let asked = sessions
        .iter()
        .find(|asked| asked.id == session)
        .ok_or_else(|| eyre!("That session is gone."))?;
    let engine = config.request_engine(asked.engine.as_deref(), asked.effort.as_deref());
    let model = engine.model_name();
    update_task(task.id, |task| {
        task.stage = Stage::Working;
        task.model = Some(model.clone());
    });
    let earlier: Vec<(&str, &str)> = asked
        .turns
        .iter()
        .map(|turn| (turn.question.as_str(), turn.answer.as_str()))
        .collect();
    let prompt = if earlier.is_empty() {
        format!("{MEETING_GUIDE}\n\n{}", question.trim())
    } else {
        format!(
            "{MEETING_GUIDE}\n\n{}",
            rewrite::follow_up_prompt(&earlier, question)
        )
    };
    let started = Instant::now();
    let (kept, drafts) = keeping(task.id);
    let made = rewrite::request(
        &engine,
        &prompt,
        transcript,
        config.vocabulary_for_requests(),
        cancel,
        &mut |fraction, parts, window_k| {
            update_task(task.id, |task| {
                task.fraction = fraction;
                task.parts = parts;
                task.window_k = window_k;
            });
        },
        &drafts,
    );
    let (answer, stopped) = match made {
        Ok(made) => (made.text, false),
        Err(error) => match stopped_with(cancel, &kept) {
            Some(text) => (text, true),
            None => return Err(error),
        },
    };
    let effort = engine.effort().map(str::to_owned);
    crate::meeting::requests::update(id, |sessions| {
        if let Some(asked) = sessions.iter_mut().find(|asked| asked.id == session) {
            asked.turns.push(FollowUp {
                question: question.trim().into(),
                answer,
                model,
                seconds: started.elapsed().as_secs(),
                created_ms: store::now_ms(),
                stopped,
            });
            if asked.effort.is_none() {
                asked.effort = effort;
            }
        }
    })?;
    Ok(())
}

/// The task's drafts, also keeping the whole answer as it streams, so a
/// stop can save what came.
fn keeping(task: u64) -> (Arc<Mutex<String>>, Drafts) {
    let kept = Arc::new(Mutex::new(String::new()));
    let shown = drafts(task);
    let keep = kept.clone();
    let drafts: Drafts = Arc::new(move |kind, text| {
        if kind == DraftKind::Result
            && let Ok(mut kept) = keep.lock()
        {
            text.clone_into(&mut kept);
        }
        shown(kind, text);
    });
    (kept, drafts)
}

/// What came before a stop, when the person stopped it and something came.
fn stopped_with(cancel: &AtomicBool, kept: &Mutex<String>) -> Option<String> {
    if !cancel.load(Ordering::Acquire) {
        return None;
    }
    let text = kept.lock().ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_keeps_what_came_and_a_failure_keeps_nothing() {
        let kept = Mutex::new("The price per seat: Anna said 12 euros".to_string());
        assert_eq!(
            stopped_with(&AtomicBool::new(true), &kept).as_deref(),
            Some("The price per seat: Anna said 12 euros")
        );
        assert_eq!(stopped_with(&AtomicBool::new(false), &kept), None);
        assert_eq!(
            stopped_with(&AtomicBool::new(true), &Mutex::new(" ".into())),
            None
        );
    }

    #[test]
    fn a_follow_up_carries_the_session_so_far() {
        let prompt = rewrite::follow_up_prompt(
            &[("Summarize it.", "Anna wants the report automated.")],
            "What did Anna promise?",
        );
        assert!(prompt.contains("<earlier_question>\nSummarize it.\n</earlier_question>"));
        assert!(prompt.contains("Anna wants the report automated."));
        assert!(prompt.ends_with("The new question: What did Anna promise?"));
    }
}
