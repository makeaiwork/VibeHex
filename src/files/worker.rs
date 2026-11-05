//! Files' queue: one worker thread runs one job at a time (transcribing a
//! file, cleaning up a version, or a request) and writes what it makes into
//! the file's record. The window reads [`tasks`] on each change and the
//! records when a job starts, moves on, or ends; each job can be stopped. Once the queue empties,
//! the built-in cleanup engine goes back to dictation's window.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

use color_eyre::eyre::{Result, eyre};

use super::rewrite::{self, Engine};
use super::store::{self, Segment, Version, VersionKind};
use super::text;
use crate::app_settings::{AppSettings, CleanupServerSettings, DictationMode, FilesSettings};
use crate::dictation::vocabulary::VocabularyEntry;
use crate::transcription::models::TranscriptionSelection;

/// What a job reads from settings, captured when it is queued.
#[derive(Clone, Debug)]
pub struct Config {
    pub files: FilesSettings,
    /// The Global mode: its prompt and OpenCode model.
    pub mode: DictationMode,
    pub server: CleanupServerSettings,
    /// Labs › Clean up on your own server is on.
    pub server_on: bool,
    pub vocabulary: Vec<VocabularyEntry>,
    /// The transcription model's name, for the Original's line.
    pub transcription_name: String,
}

impl Config {
    pub fn from_settings(settings: &AppSettings) -> Self {
        let selection = settings
            .files
            .selection(&settings.transcription)
            .unwrap_or_else(|| settings.transcription.clone());
        Self {
            files: settings.files.clone(),
            mode: settings.dictation_processing.default_mode.clone(),
            server: settings.dictation_processing.server.clone(),
            server_on: settings.labs.ai_cleanup_server,
            vocabulary: settings.vocabulary.clone(),
            transcription_name: crate::transcription::models::definition(selection.model)
                .name
                .to_string(),
        }
    }

    fn vocabulary(&self) -> &[VocabularyEntry] {
        if self.files.use_vocabulary {
            &self.vocabulary
        } else {
            &[]
        }
    }

    /// Where cleanup runs; `choice` overrides Settings' choice.
    pub fn engine(&self, choice: Option<&str>) -> Engine {
        Engine::resolve(
            choice.or(self.files.cleanup_engine.as_deref()),
            &self.server,
            self.server_on,
            &self.mode,
        )
    }

    fn cleanup_prompt(&self) -> &str {
        if self.files.use_default_prompt {
            &self.mode.post_processing.prompt
        } else {
            &self.files.prompt
        }
    }
}

/// One job on one file.
#[derive(Clone, Debug)]
pub enum Job {
    /// Transcribes the file. `again` adds another Original with `selection`
    /// (Transcribe again); otherwise it is the file's first, which cleanup
    /// may follow.
    Transcribe {
        selection: Option<TranscriptionSelection>,
        model_name: String,
        again: bool,
    },
    /// Cleans up version `from`; `engine` overrides Settings' choice (Clean
    /// up again).
    Cleanup { from: u32, engine: Option<String> },
    /// Applies `prompt` to version `from`, as the tab `name`.
    Request {
        from: u32,
        prompt: String,
        name: String,
    },
}

/// Where a job is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Queued,
    /// Decoding the file's audio.
    Extracting,
    Transcribing,
    CleaningUp,
    /// A request.
    Working,
}

/// A job as the window shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: u64,
    pub record: String,
    pub stage: Stage,
    /// From 0 to 1.
    pub fraction: f32,
    /// A request worked in three steps over this many parts.
    pub parts: Option<usize>,
    /// The tab a request or cleanup will add.
    pub name: Option<String>,
    /// What it adds: an Original, a Cleanup, or a request.
    pub kind: VersionKind,
    /// The version a cleanup or request works on.
    pub from: Option<u32>,
    /// The model it runs on, once it starts.
    pub model: Option<String>,
    /// The built-in engine's window in K tokens, once it is chosen.
    pub window_k: Option<u32>,
}

struct Queued {
    task: Task,
    job: Job,
    config: Config,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct State {
    queue: VecDeque<Queued>,
    next: u64,
    started: bool,
    /// The last failure per record, until the window dismisses it.
    errors: Vec<(String, String)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    queue: VecDeque::new(),
    next: 1,
    started: false,
    errors: Vec::new(),
});
static WORK: Condvar = Condvar::new();
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

/// The queue, the running job first.
pub fn tasks() -> Vec<Task> {
    state()
        .queue
        .iter()
        .map(|queued| queued.task.clone())
        .collect()
}

/// The last failure for `record`.
pub fn error(record: &str) -> Option<String> {
    state()
        .errors
        .iter()
        .find(|(id, _)| id == record)
        .map(|(_, error)| error.clone())
}

pub fn dismiss_error(record: &str) {
    let mut state = state();
    state.errors.retain(|(id, _)| id != record);
    changed(state);
}

/// Queues `job` on `record` and returns its task id, 0 when Labs › Files
/// is off. `next` puts it right after the running job.
pub fn submit(record: &str, job: Job, config: Config, next: bool) -> u64 {
    // Labs › Files off: no job runs.
    if !crate::labs::files() {
        tracing::info!("Files is off in Labs; the job wasn't queued");
        return 0;
    }
    let mut state = state();
    let id = state.next;
    state.next += 1;
    let (name, kind, from) = match &job {
        Job::Request { name, from, .. } => (Some(name.clone()), VersionKind::Request, Some(*from)),
        Job::Cleanup { from, .. } => (Some("Cleanup".into()), VersionKind::Cleanup, Some(*from)),
        Job::Transcribe { .. } => (None, VersionKind::Original, None),
    };
    let queued = Queued {
        task: Task {
            id,
            record: record.into(),
            stage: Stage::Queued,
            fraction: 0.0,
            parts: None,
            name,
            kind,
            from,
            model: None,
            window_k: None,
        },
        job,
        config,
        cancel: Arc::new(AtomicBool::new(false)),
    };
    if next && !state.queue.is_empty() {
        state.queue.insert(1, queued);
    } else {
        state.queue.push_back(queued);
    }
    state.errors.retain(|(id, _)| id != record);
    if !state.started {
        state.started = true;
        let spawned = std::thread::Builder::new()
            .name("hex-files".into())
            .spawn(run);
        if let Err(error) = spawned {
            tracing::error!(%error, "couldn't start the Files worker");
            state.started = false;
        }
    }
    WORK.notify_all();
    changed(state);
    id
}

/// Stops task `id`: a queued one leaves the queue, a running one ends at
/// its next step.
pub fn cancel(id: u64) {
    let mut state = state();
    if let Some(index) = state.queue.iter().position(|queued| queued.task.id == id) {
        if index == 0 {
            state.queue[0].cancel.store(true, Ordering::Release);
        } else {
            state.queue.remove(index);
        }
    }
    changed(state);
}

/// Stops every job on `record` (it is being deleted).
pub fn cancel_record(record: &str) {
    let ids: Vec<u64> = tasks()
        .into_iter()
        .filter(|task| task.record == record)
        .map(|task| task.id)
        .collect();
    for id in ids {
        cancel(id);
    }
}

fn update_task(id: u64, change: impl FnOnce(&mut Task)) {
    let mut state = state();
    if let Some(queued) = state.queue.iter_mut().find(|queued| queued.task.id == id) {
        change(&mut queued.task);
    }
    changed(state);
}

fn run() {
    loop {
        let (task, job, config, cancel) = {
            let mut state = state();
            while state.queue.is_empty() {
                state = WORK
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            let front = &state.queue[0];
            (
                front.task.clone(),
                front.job.clone(),
                front.config.clone(),
                front.cancel.clone(),
            )
        };
        if crate::desktop::process_exit::quitting() {
            return;
        }
        let result = run_job(&task, &job, &config, &cancel);
        let mut state = state();
        state.queue.retain(|queued| queued.task.id != task.id);
        match result {
            Ok(()) => {}
            Err(_) if cancel.load(Ordering::Acquire) => {}
            Err(error) => {
                tracing::warn!(record = task.record, "a Files job failed: {error:#}");
                state.errors.retain(|(id, _)| id != &task.record);
                state
                    .errors
                    .push((task.record.clone(), format!("{error:#}")));
            }
        }
        let idle = state.queue.is_empty();
        changed(state);
        if idle {
            crate::dictation::cleanup_server::engine::narrow();
        }
    }
}

fn run_job(task: &Task, job: &Job, config: &Config, cancel: &AtomicBool) -> Result<()> {
    match job {
        Job::Transcribe {
            selection,
            model_name,
            again,
        } => transcribe(task, selection.clone(), model_name, *again, config, cancel),
        Job::Cleanup { from, engine } => cleanup(task, *from, engine.as_deref(), config, cancel),
        Job::Request { from, prompt, name } => request(task, *from, prompt, name, config, cancel),
    }
}

fn transcribe(
    task: &Task,
    selection: Option<TranscriptionSelection>,
    model_name: &str,
    again: bool,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let record = store::load(&task.record)?;
    update_task(task.id, |task| task.stage = Stage::Extracting);
    let wav = temporary_wav(&task.record)?;
    let decoded = super::decode::to_wav(&record.path, &wav);
    let result = decoded.and_then(|duration_ms| {
        store::update(&task.record, |record| record.duration_ms = duration_ms)?;
        update_task(task.id, |task| task.stage = Stage::Transcribing);
        let started = Instant::now();
        let entries = crate::meeting::transcript::transcribe_wav(
            &wav,
            selection,
            config.files.use_vocabulary,
            cancel,
            &mut |done_ms| {
                let fraction = done_ms as f32 / duration_ms.max(1) as f32;
                update_task(task.id, |task| task.fraction = fraction.min(1.0));
            },
        )?;
        Ok((entries, started.elapsed().as_secs()))
    });
    let _ = std::fs::remove_file(&wav);
    let (entries, seconds) = match result {
        Ok(done) => done,
        Err(error) => {
            if !again && !cancel.load(Ordering::Acquire) {
                store::update(&task.record, |record| {
                    record.error = Some(format!("{error:#}"));
                })?;
            }
            return Err(error);
        }
    };
    let segments: Vec<Segment> = entries
        .into_iter()
        .map(|entry| Segment {
            start_ms: entry.start_ms,
            end_ms: entry.end_ms,
            text: entry.text,
        })
        .collect();
    if segments.is_empty() {
        let error = eyre!("No speech was heard in this file.");
        if !again {
            store::update(&task.record, |record| {
                record.error = Some(error.to_string());
            })?;
        }
        return Err(error);
    }
    let added = store::update(&task.record, |record| {
        record.error = None;
        let name = if again {
            record.unique_name(&format!("Original ({})", short_name(model_name)))
        } else {
            "Original".into()
        };
        record.push(Version {
            kind: VersionKind::Original,
            name,
            text: text::plain(&segments),
            segments,
            model: model_name.into(),
            seconds,
            created_ms: store::now_ms(),
            ..Version::default()
        })
    })?;
    if let Some(from) = added
        && !again
        && config.files.cleanup_after
    {
        submit(
            &task.record,
            Job::Cleanup { from, engine: None },
            config.clone(),
            true,
        );
    }
    Ok(())
}

/// `Whisper large-v3-turbo` → `Whisper`, `Parakeet v3` as it is, so a tab
/// stays short and still tells models apart.
fn short_name(model: &str) -> String {
    let mut words = model.split_whitespace();
    let first = words.next().unwrap_or(model);
    match words.next() {
        Some(second) if second.chars().count() <= 5 => format!("{first} {second}"),
        _ => first.into(),
    }
}

fn temporary_wav(record: &str) -> Result<PathBuf> {
    let dir = store::dir()?.join("decoding");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(format!("{record}.wav")))
}

/// The text a version is worked from: an Original without its times.
fn source(record: &store::FileRecord, from: u32) -> Result<String> {
    let version = record
        .version(from)
        .ok_or_else(|| eyre!("That version is gone."))?;
    Ok(if version.segments.is_empty() || version.edited {
        version.text.clone()
    } else {
        text::plain(&version.segments)
    })
}

fn cleanup(
    task: &Task,
    from: u32,
    choice: Option<&str>,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let record = store::load(&task.record)?;
    let input = source(&record, from)?;
    let engine = config.engine(choice);
    update_task(task.id, |task| {
        task.stage = Stage::CleaningUp;
        task.model = Some(engine.model_name());
        task.window_k = rewrite::cleanup_window_k(&engine);
    });
    let started = Instant::now();
    let instructions = rewrite::cleanup_instructions(config.cleanup_prompt(), config.vocabulary());
    let made = rewrite::clean(&engine, &instructions, &input, cancel, &mut |fraction| {
        update_task(task.id, |task| task.fraction = fraction);
    })?;
    let model = engine.model_name();
    store::update(&task.record, |record| {
        let first = !record
            .versions
            .iter()
            .any(|version| version.kind == VersionKind::Cleanup);
        let name = if first && choice.is_none() {
            "Cleanup".to_string()
        } else {
            record.unique_name(&format!("Cleanup ({model})"))
        };
        record.push(Version {
            kind: VersionKind::Cleanup,
            name,
            text: made.text,
            model,
            from: Some(from),
            window_k: made.window_k,
            seconds: started.elapsed().as_secs(),
            created_ms: store::now_ms(),
            failed: made.failed,
            ..Version::default()
        });
    })?;
    Ok(())
}

fn request(
    task: &Task,
    from: u32,
    prompt: &str,
    name: &str,
    config: &Config,
    cancel: &AtomicBool,
) -> Result<()> {
    let record = store::load(&task.record)?;
    let input = source(&record, from)?;
    let engine = config.engine(config.files.request_engine.as_deref());
    update_task(task.id, |task| {
        task.stage = Stage::Working;
        task.model = Some(engine.model_name());
    });
    let started = Instant::now();
    let made = rewrite::request(
        &engine,
        prompt,
        &input,
        config.vocabulary(),
        cancel,
        &mut |fraction, parts, window_k| {
            update_task(task.id, |task| {
                task.fraction = fraction;
                task.parts = parts;
                task.window_k = window_k;
            });
        },
    )?;
    let model = engine.model_name();
    store::update(&task.record, |record| {
        let name = record.unique_name(name);
        record.push(Version {
            kind: VersionKind::Request,
            name,
            text: made.text,
            model,
            from: Some(from),
            prompt: Some(prompt.into()),
            window_k: made.window_k,
            parts: made.parts,
            seconds: started.elapsed().as_secs(),
            created_ms: store::now_ms(),
            failed: made.failed,
            ..Version::default()
        });
    })?;
    Ok(())
}

/// The tab name for a request: its chip's label, else its first words.
pub fn request_name(prompt: &str, quick: &[crate::app_settings::QuickPrompt]) -> String {
    if let Some(chip) = quick
        .iter()
        .find(|chip| chip.prompt.trim() == prompt.trim())
    {
        return chip.label.clone();
    }
    let words: Vec<&str> = prompt.split_whitespace().take(3).collect();
    let mut name = words.join(" ");
    name = name
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .to_string();
    if prompt.split_whitespace().count() > 3 {
        name.push('…');
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Request".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_settings::QuickPrompt;

    #[test]
    fn requests_are_named_by_their_chip_or_first_words() {
        let quick = vec![QuickPrompt {
            label: "Summary".into(),
            prompt: "Write a summary".into(),
        }];
        assert_eq!(request_name("Write a summary", &quick), "Summary");
        assert_eq!(
            request_name("сделай из этого пост для Telegram", &quick),
            "Сделай из этого…"
        );
        assert_eq!(request_name("переведи", &quick), "Переведи");
        assert_eq!(request_name("  ", &quick), "Request");
    }

    #[test]
    fn transcription_tabs_keep_a_short_model_name() {
        assert_eq!(short_name("Whisper large-v3-turbo"), "Whisper");
        assert_eq!(short_name("Parakeet v3"), "Parakeet v3");
        assert_eq!(short_name("Parakeet Ultra"), "Parakeet Ultra");
        assert_eq!(short_name("Cohere Transcribe"), "Cohere");
    }
}
