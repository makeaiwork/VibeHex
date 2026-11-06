//! Codex, OpenAI's agent CLI already installed and signed in on this
//! computer, as a cleanup engine: dictations (and Files and Meetings text)
//! rewritten on the person's own ChatGPT plan.
//!
//! Hex starts `codex app-server` on first use and speaks its JSON-RPC over
//! the child's stdin and stdout, one JSON object per line. The process stays
//! warm for [`IDLE`] after its last request, then exits. Every request is a
//! new ephemeral, read-only thread whose base instructions are Hex's; Hex's
//! process switches off the person's hooks, MCP servers, memories, plugins,
//! apps, shell, web search, project instructions, and `notify` command
//! ([`ISOLATION`]), so Codex sees only the text and keeps nothing of it. Hex
//! never sees a password or key: Codex keeps its own sign-in in `~/.codex`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak, mpsc};
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::dictation::cleanup_pieces::Streamed;

/// How long the app-server stays running after its last request.
const IDLE: Duration = Duration::from_secs(10 * 60);
/// How often a waiting request checks cancellation, and the idle check's period.
const POLL: Duration = Duration::from_millis(50);
const IDLE_CHECK: Duration = Duration::from_secs(30);
/// How long a status read (account, models, plan usage) may take.
pub const STATUS_DEADLINE: Duration = Duration::from_secs(20);
/// What features say when no `codex` executable is found.
pub const NOT_INSTALLED: &str = "Codex isn't installed";
/// The installer command Output shows.
pub const INSTALL_COMMAND: &str = if cfg!(target_os = "macos") {
    "brew install codex"
} else {
    "npm install -g @openai/codex"
};

/// Overrides for Hex's own app-server: nothing of the person's Codex setup
/// that could act, remember, read a project, or make a sound. Feature flags
/// go through `-c` because `--disable` refuses a flag a Codex version
/// doesn't know.
const ISOLATION: &[&str] = &[
    "notify=[]",
    "mcp_servers={}",
    "project_doc_max_bytes=0",
    "web_search=\"disabled\"",
    "features.hooks=false",
    "features.memories=false",
    "features.apps=false",
    "features.plugins=false",
    "features.multi_agent=false",
    "features.shell_tool=false",
    "features.unified_exec=false",
    "features.computer_use=false",
    "features.browser_use=false",
    "features.in_app_browser=false",
    "features.image_generation=false",
    "features.goals=false",
    "features.code_mode_host=false",
    "features.js_repl=false",
];

/// The signed-in account, as Codex reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Account {
    /// Signed in with ChatGPT; `plan` is its plan type, such as `pro`.
    ChatGpt {
        email: String,
        plan: String,
    },
    /// Codex uses an OpenAI API key of its own.
    ApiKey,
    /// Another provider Codex is set up for, such as Amazon Bedrock.
    Other,
    SignedOut,
}

/// One model Codex offers, from `model/list`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub description: String,
    pub efforts: Vec<String>,
    pub default_effort: String,
    pub default: bool,
}

/// One plan-usage window, such as five hours or a week.
#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub window_minutes: u64,
    pub used_percent: f32,
    /// Unix seconds when the window resets.
    pub resets_at: Option<i64>,
}

/// What Output shows about Codex.
#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub executable: PathBuf,
    pub version: String,
    pub account: Account,
    pub models: Vec<Model>,
    pub limits: Vec<Limit>,
    /// The plan's limit is reached: requests fail until it resets.
    pub limited: bool,
}

/// The `codex` executable: on `PATH`, then where npm, Homebrew, Bun, Volta,
/// and nvm put it, since an app opened from the Finder gets a short `PATH`.
pub fn find() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["codex.exe", "codex.cmd"]
    } else {
        &["codex"]
    };
    search_dirs()
        .into_iter()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|path| path.is_file())
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    let home = dirs::home_dir();
    #[cfg(unix)]
    {
        dirs.push("/opt/homebrew/bin".into());
        dirs.push("/usr/local/bin".into());
        if let Some(home) = &home {
            for dir in [".local/bin", ".npm-global/bin", ".bun/bin", ".volta/bin"] {
                dirs.push(home.join(dir));
            }
            // Every nvm Node, newest name first.
            let nvm = home.join(".nvm/versions/node");
            let mut nodes: Vec<PathBuf> = std::fs::read_dir(&nvm)
                .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
                .unwrap_or_default();
            nodes.sort();
            dirs.extend(nodes.into_iter().rev().map(|node| node.join("bin")));
        }
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            dirs.push(PathBuf::from(appdata).join("npm"));
        }
        if let Some(home) = &home {
            dirs.push(home.join(".bun").join("bin"));
        }
    }
    #[cfg(not(any(unix, windows)))]
    let _ = home;
    dirs
}

/// `PATH` for the child: the executable's own folder first, where npm's
/// `codex` finds the `node` it was installed with, then Hex's, then
/// Homebrew's.
fn child_path(executable: &Path) -> std::ffi::OsString {
    let mut paths: Vec<PathBuf> = executable
        .parent()
        .map(Path::to_owned)
        .into_iter()
        .collect();
    if let Ok(target) = std::fs::canonicalize(executable)
        && let Some(dir) = target.parent()
    {
        paths.push(dir.to_owned());
    }
    if let Some(current) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&current));
    }
    #[cfg(unix)]
    for extra in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        paths.push(extra.into());
    }
    std::env::join_paths(paths).unwrap_or_default()
}

/// Account, models, and plan usage, starting Codex if it isn't running.
pub fn status(cancelled: &AtomicBool) -> Result<Status> {
    let until = Instant::now() + STATUS_DEADLINE;
    let process = running(until, cancelled)?;
    let account = process.request(
        "account/read",
        json!({ "refreshToken": false }),
        until,
        cancelled,
    )?;
    let account = read_account(&account);
    let mut models = Vec::new();
    let mut limits = Vec::new();
    let mut limited = false;
    if account != Account::SignedOut {
        let listed = process.request("model/list", json!({}), until, cancelled)?;
        models = read_models(&listed);
        // Plan usage is only for a ChatGPT plan; an API key has none.
        if matches!(account, Account::ChatGpt { .. })
            && let Ok(usage) =
                process.request("account/rateLimits/read", Value::Null, until, cancelled)
        {
            (limits, limited) = read_limits(&usage);
        }
    }
    Ok(Status {
        executable: process.executable.clone(),
        version: process.version.get().cloned().unwrap_or_default(),
        account,
        models,
        limits,
        limited,
    })
}

/// Starts signing in with ChatGPT: the page to open in the browser. Codex
/// finishes the sign-in itself; [`status`] shows the account after.
pub fn sign_in(cancelled: &AtomicBool) -> Result<String> {
    let until = Instant::now() + STATUS_DEADLINE;
    let process = running(until, cancelled)?;
    let started = process.request(
        "account/login/start",
        json!({ "type": "chatgpt" }),
        until,
        cancelled,
    )?;
    started
        .get("authUrl")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| eyre!("Codex didn't offer a sign-in page"))
}

/// One answer: `instructions` as the thread's base instructions, `input` as
/// the message, on `model` (empty: Codex's default) thinking at `effort`
/// (empty: the model's default). The answer streams to `streamed`.
pub fn generate(
    model: &str,
    effort: &str,
    instructions: &str,
    input: &str,
    deadline: Duration,
    cancelled: &AtomicBool,
    streamed: Option<Streamed>,
) -> Result<String> {
    let until = Instant::now() + deadline;
    let process = running(until, cancelled)?;
    let started = process.request(
        "thread/start",
        json!({
            "model": non_empty(model),
            "cwd": workspace()?,
            "approvalPolicy": "never",
            "sandbox": "read-only",
            "ephemeral": true,
            "baseInstructions": instructions,
        }),
        until,
        cancelled,
    )?;
    let thread = started
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .ok_or_else(|| eyre!("Codex started no conversation"))?
        .to_owned();
    let events = process.follow(&thread);
    let turn = process.request(
        "turn/start",
        json!({
            "threadId": thread,
            "input": [{ "type": "text", "text": input, "text_elements": [] }],
            "effort": non_empty(effort),
        }),
        until,
        cancelled,
    )?;
    let turn = turn
        .pointer("/turn/id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut text = String::new();
    let mut chars = 0;
    loop {
        if cancelled.load(Ordering::Acquire) {
            process.interrupt(&thread, &turn);
            return Err(eyre!("the request to Codex was cancelled"));
        }
        let remaining = until.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            process.interrupt(&thread, &turn);
            return Err(eyre!(
                "Codex didn't answer within {} seconds",
                deadline.as_secs()
            ));
        }
        match events.receiver.recv_timeout(remaining.min(POLL)) {
            Ok(Event::Delta(piece)) => {
                chars += piece.chars().count();
                text.push_str(&piece);
                if let Some(streamed) = &streamed {
                    streamed(chars, &text);
                }
            }
            Ok(Event::Completed(turn)) => return finished(&turn, &text),
            Ok(Event::Failed(why)) => return Err(eyre!("Codex: {why}")),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(eyre!("Codex stopped")),
        }
    }
}

/// The answer of a finished turn: its last agent message, else what streamed.
fn finished(turn: &Value, streamed: &str) -> Result<String> {
    match turn.get("status").and_then(Value::as_str) {
        Some("completed") => {}
        Some("interrupted") => return Err(eyre!("Codex stopped the answer")),
        _ => {
            let error = turn.get("error").cloned().unwrap_or(Value::Null);
            return Err(eyre!("{}", error_message(&error)));
        }
    }
    let said = turn
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("agentMessage"))
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .next_back()
        .unwrap_or(streamed);
    let text = super::clean(said);
    if text.is_empty() {
        Err(eyre!("Codex returned empty text"))
    } else {
        Ok(text)
    }
}

/// What a turn's or a request's error says: a usage limit and a lost sign-in
/// by name, else the server's message, which Codex passes on as JSON text.
fn error_message(error: &Value) -> String {
    match error.get("codexErrorInfo").and_then(Value::as_str) {
        Some("usageLimitExceeded" | "rateLimitExceeded") => {
            return "your ChatGPT plan's Codex limit is reached".into();
        }
        Some("unauthorized") => return "sign in to Codex again".into(),
        _ => {}
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Codex couldn't answer");
    let inner = serde_json::from_str::<Value>(message).ok();
    let message = inner
        .as_ref()
        .and_then(|value| value.pointer("/error/message").and_then(Value::as_str))
        .unwrap_or(message);
    match message.char_indices().nth(super::ERROR_EXCERPT) {
        Some((cut, _)) => format!("{}…", &message[..cut]),
        None => message.into(),
    }
}

fn non_empty(value: &str) -> Value {
    match value.trim() {
        "" => Value::Null,
        value => Value::String(value.into()),
    }
}

/// An empty folder Hex owns, where Codex's threads run read-only.
fn workspace() -> Result<PathBuf> {
    let dir = crate::app_paths::support_dir()?.join("codex");
    std::fs::create_dir_all(&dir)
        .map_err(|error| eyre!("couldn't create {}: {error}", dir.display()))?;
    Ok(dir)
}

fn read_account(response: &Value) -> Account {
    let account = response.get("account").unwrap_or(&Value::Null);
    let text = |key: &str| {
        account
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    match account.get("type").and_then(Value::as_str) {
        Some("chatgpt") => Account::ChatGpt {
            email: text("email"),
            plan: text("planType"),
        },
        Some("apiKey") => Account::ApiKey,
        Some(_) => Account::Other,
        None => Account::SignedOut,
    }
}

fn read_models(response: &Value) -> Vec<Model> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Listed {
        id: String,
        #[serde(default)]
        display_name: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        hidden: bool,
        #[serde(default)]
        supported_reasoning_efforts: Vec<Effort>,
        #[serde(default)]
        default_reasoning_effort: String,
        #[serde(default)]
        is_default: bool,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Effort {
        reasoning_effort: String,
    }
    let listed: Vec<Listed> = response
        .get("data")
        .cloned()
        .and_then(|data| serde_json::from_value(data).ok())
        .unwrap_or_default();
    listed
        .into_iter()
        .filter(|model| !model.hidden)
        .map(|model| Model {
            name: if model.display_name.is_empty() {
                model.id.clone()
            } else {
                model.display_name
            },
            id: model.id,
            description: model.description.trim_end_matches('.').to_owned(),
            efforts: model
                .supported_reasoning_efforts
                .into_iter()
                .map(|effort| effort.reasoning_effort)
                .collect(),
            default_effort: model.default_reasoning_effort,
            default: model.is_default,
        })
        .collect()
}

fn read_limits(response: &Value) -> (Vec<Limit>, bool) {
    let limits = response.get("rateLimits").unwrap_or(&Value::Null);
    let windows = ["primary", "secondary"]
        .into_iter()
        .filter_map(|key| limits.get(key).filter(|window| window.is_object()))
        .filter_map(|window| {
            Some(Limit {
                window_minutes: window.get("windowDurationMins")?.as_u64()?,
                used_percent: window.get("usedPercent")?.as_f64()? as f32,
                resets_at: window.get("resetsAt").and_then(Value::as_i64),
            })
        })
        .collect();
    let reached = limits
        .get("rateLimitReachedType")
        .is_some_and(|reached| !reached.is_null());
    (windows, reached)
}

/// What the reader passes to a request waiting on a thread.
enum Event {
    Delta(String),
    Completed(Value),
    Failed(String),
}

/// A thread's events while a request follows it; dropping it unsubscribes.
struct Following {
    process: Arc<Process>,
    thread: String,
    receiver: mpsc::Receiver<Event>,
}

impl Drop for Following {
    fn drop(&mut self) {
        if let Ok(mut threads) = self.process.threads.lock() {
            threads.remove(&self.thread);
        }
        // An ephemeral thread lives until unsubscribed; nothing waits for it.
        let _ = self.process.send(&json!({
            "id": self.process.next_id(),
            "method": "thread/unsubscribe",
            "params": { "threadId": self.thread },
        }));
    }
}

static RUNNING: Mutex<Option<Arc<Process>>> = Mutex::new(None);

/// The warm app-server, started when there is none.
fn running(until: Instant, cancelled: &AtomicBool) -> Result<Arc<Process>> {
    let mut running = RUNNING
        .lock()
        .map_err(|_| eyre!("Codex's connection is unavailable"))?;
    if let Some(process) = running
        .as_ref()
        .filter(|process| process.alive.load(Ordering::Acquire))
    {
        process.touch();
        return Ok(process.clone());
    }
    let process = Process::start(until, cancelled)?;
    *running = Some(process.clone());
    Ok(process)
}

/// One running `codex app-server`.
struct Process {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    next: AtomicU64,
    /// Requests waiting for their response, by id.
    waiting: Mutex<HashMap<u64, mpsc::Sender<Result<Value, String>>>>,
    /// Threads a request follows, by id.
    threads: Mutex<HashMap<String, mpsc::Sender<Event>>>,
    used: Mutex<Instant>,
    alive: AtomicBool,
    executable: PathBuf,
    /// Codex's version, from the handshake.
    version: OnceLock<String>,
}

impl Process {
    fn start(until: Instant, cancelled: &AtomicBool) -> Result<Arc<Self>> {
        let executable = find().ok_or_else(|| eyre!(NOT_INSTALLED))?;
        let mut command = crate::desktop::background_process::command(&executable);
        command.arg("app-server");
        for setting in ISOLATION {
            command.arg("-c").arg(setting);
        }
        let mut child = command
            .env("PATH", child_path(&executable))
            .current_dir(workspace()?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| eyre!("couldn't start Codex: {error}"))?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            return Err(eyre!("couldn't connect to Codex"));
        };
        let process = Arc::new(Self {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            next: AtomicU64::new(1),
            waiting: Mutex::new(HashMap::new()),
            threads: Mutex::new(HashMap::new()),
            used: Mutex::new(Instant::now()),
            alive: AtomicBool::new(true),
            executable,
            version: OnceLock::new(),
        });
        spawn_reader(&process, stdout)?;
        let initialized = process
            .request(
                "initialize",
                json!({
                    "clientInfo": {
                        "name": "hex",
                        "title": "VibeHex",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": null,
                }),
                until,
                cancelled,
            )
            .and_then(|initialized| {
                process.send(&json!({ "method": "initialized" }))?;
                Ok(initialized)
            });
        let initialized = match initialized {
            Ok(initialized) => initialized,
            Err(error) => {
                process.stop();
                return Err(error);
            }
        };
        // `userAgent` is `hex/<codex version> (…)`.
        let version = initialized
            .get("userAgent")
            .and_then(Value::as_str)
            .and_then(|agent| agent.split_once('/'))
            .map(|(_, rest)| rest.split(' ').next().unwrap_or_default().to_owned())
            .unwrap_or_default();
        let _ = process.version.set(version);
        spawn_idle_check(&process);
        tracing::info!(executable = %process.executable.display(), "Codex app-server started");
        Ok(process)
    }

    fn next_id(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    fn touch(&self) {
        if let Ok(mut used) = self.used.lock() {
            *used = Instant::now();
        }
    }

    fn send(&self, message: &Value) -> Result<()> {
        let mut line = serde_json::to_vec(message)?;
        line.push(b'\n');
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| eyre!("Codex's connection is unavailable"))?;
        stdin
            .write_all(&line)
            .and_then(|()| stdin.flush())
            .map_err(|_| eyre!("Codex stopped"))
    }

    /// One request and its result, within `until` unless cancelled.
    fn request(
        &self,
        method: &str,
        params: Value,
        until: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Value> {
        self.touch();
        let id = self.next_id();
        let (sender, receiver) = mpsc::channel();
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.insert(id, sender);
        }
        let mut message = json!({ "id": id, "method": method });
        if !params.is_null() {
            message["params"] = params;
        }
        self.send(&message)?;
        self.wait(id, &receiver, until, cancelled)
    }

    fn wait(
        &self,
        id: u64,
        receiver: &mpsc::Receiver<Result<Value, String>>,
        until: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Value> {
        let forget = || {
            if let Ok(mut waiting) = self.waiting.lock() {
                waiting.remove(&id);
            }
        };
        loop {
            if cancelled.load(Ordering::Acquire) {
                forget();
                return Err(eyre!("the request to Codex was cancelled"));
            }
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                forget();
                return Err(eyre!("Codex didn't answer in time"));
            }
            match receiver.recv_timeout(remaining.min(POLL)) {
                Ok(result) => return result.map_err(|why| eyre!("Codex: {why}")),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(eyre!("Codex stopped")),
            }
        }
    }

    /// Receives `thread`'s events until the returned value drops.
    fn follow(self: &Arc<Self>, thread: &str) -> Following {
        let (sender, receiver) = mpsc::channel();
        if let Ok(mut threads) = self.threads.lock() {
            threads.insert(thread.to_owned(), sender);
        }
        Following {
            process: self.clone(),
            thread: thread.to_owned(),
            receiver,
        }
    }

    /// Asks Codex to stop a turn; nothing waits for the answer.
    fn interrupt(&self, thread: &str, turn: &str) {
        if turn.is_empty() {
            return;
        }
        let _ = self.send(&json!({
            "id": self.next_id(),
            "method": "turn/interrupt",
            "params": { "threadId": thread, "turnId": turn },
        }));
    }

    /// Routes one message from Codex.
    fn route(&self, message: Value) {
        let method = message.get("method").and_then(Value::as_str);
        match (message.get("id"), method) {
            // Codex asking permission or input: Hex grants nothing.
            (Some(id), Some(_)) => {
                let _ = self.send(&json!({
                    "id": id,
                    "error": { "code": -32601, "message": "Hex allows no tools" },
                }));
            }
            (Some(id), None) => {
                let Some(id) = id.as_u64() else { return };
                let Some(sender) = self.waiting.lock().ok().and_then(|mut w| w.remove(&id)) else {
                    return;
                };
                let result = match message.get("error") {
                    Some(error) => Err(error_message(error)),
                    None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
                };
                let _ = sender.send(result);
            }
            (None, Some(method)) => self.notified(method, message.get("params")),
            (None, None) => {}
        }
    }

    fn notified(&self, method: &str, params: Option<&Value>) {
        let Some(params) = params else { return };
        let event = match method {
            "item/agentMessage/delta" => params
                .get("delta")
                .and_then(Value::as_str)
                .map(|delta| Event::Delta(delta.to_owned())),
            "turn/completed" => params.get("turn").cloned().map(Event::Completed),
            // An error Codex retries by itself is only a warning.
            "error" if params.get("willRetry").and_then(Value::as_bool) != Some(true) => params
                .get("error")
                .map(|error| Event::Failed(error_message(error))),
            _ => None,
        };
        let (Some(event), Some(thread)) = (event, params.get("threadId").and_then(Value::as_str))
        else {
            return;
        };
        if let Some(sender) = self
            .threads
            .lock()
            .ok()
            .and_then(|t| t.get(thread).cloned())
        {
            let _ = sender.send(event);
        }
    }

    /// Ends the process and fails whatever waits on it.
    fn stop(&self) {
        self.alive.store(false, Ordering::Release);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.clear();
        }
        if let Ok(mut threads) = self.threads.lock() {
            threads.clear();
        }
    }
}

/// Reads Codex's messages until it exits, then fails whatever waits on it.
fn spawn_reader(process: &Arc<Process>, stdout: ChildStdout) -> Result<()> {
    let process = process.clone();
    std::thread::Builder::new()
        .name("hex-codex-reader".into())
        .spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Ok(message) = serde_json::from_str::<Value>(&line) {
                    process.route(message);
                }
            }
            tracing::info!("Codex app-server exited");
            process.stop();
        })
        .map(drop)
        .map_err(|error| eyre!("couldn't read from Codex: {error}"))
}

/// Ends the process after [`IDLE`] without a request or a followed thread.
fn spawn_idle_check(process: &Arc<Process>) {
    let process: Weak<Process> = Arc::downgrade(process);
    let _ = std::thread::Builder::new()
        .name("hex-codex-idle".into())
        .spawn(move || {
            loop {
                std::thread::sleep(IDLE_CHECK);
                let Some(process) = process.upgrade() else {
                    return;
                };
                if !process.alive.load(Ordering::Acquire) {
                    return;
                }
                let idle = process.used.lock().is_ok_and(|used| used.elapsed() >= IDLE);
                let following = process
                    .threads
                    .lock()
                    .is_ok_and(|threads| !threads.is_empty());
                if idle && !following {
                    tracing::info!("Codex app-server stopped after idling");
                    if let Ok(mut running) = RUNNING.lock()
                        && running
                            .as_ref()
                            .is_some_and(|running| Arc::ptr_eq(running, &process))
                    {
                        *running = None;
                    }
                    process.stop();
                    return;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real cleanup on the Codex CLI signed in on this computer:
    /// `cargo nextest run --run-ignored only answers_on_the_installed_codex`.
    #[test]
    #[ignore = "needs the Codex CLI, signed in"]
    fn answers_on_the_installed_codex() {
        let cancelled = AtomicBool::new(false);
        let status = status(&cancelled).unwrap();
        eprintln!("{status:?}");
        assert!(!status.version.is_empty());
        assert!(!status.models.is_empty());
        let streamed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = streamed.clone();
        let answer = generate(
            "",
            "low",
            "You clean up dictated text: punctuation, casing, no fillers. Reply with the cleaned text only.",
            "um so like i think we should uh ship it on friday",
            Duration::from_secs(60),
            &cancelled,
            Some(Arc::new(move |chars, _| seen.store(chars, Ordering::Relaxed))),
        )
        .unwrap();
        eprintln!("{answer}");
        assert!(answer.to_lowercase().contains("friday"));
        assert!(streamed.load(Ordering::Relaxed) > 0);
        // A cancelled request ends at once.
        let cancelled = AtomicBool::new(true);
        assert!(
            generate(
                "",
                "low",
                "x",
                "y",
                Duration::from_secs(60),
                &cancelled,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn a_failed_turn_says_the_server_message() {
        let error = json!({
            "message": "{\"type\":\"error\",\"error\":{\"message\":\"Unsupported value: 'minimal'\"},\"status\":400}",
            "codexErrorInfo": "other",
        });
        assert_eq!(error_message(&error), "Unsupported value: 'minimal'");
        assert_eq!(
            error_message(&json!({ "message": "x", "codexErrorInfo": "usageLimitExceeded" })),
            "your ChatGPT plan's Codex limit is reached"
        );
    }

    #[test]
    fn a_completed_turn_answers_with_its_last_message() {
        let turn = json!({
            "status": "completed",
            "items": [
                { "type": "agentMessage", "text": "thinking aloud" },
                { "type": "reasoning" },
                { "type": "agentMessage", "text": "```\nI think we should ship it on Friday.\n```" },
            ],
        });
        assert_eq!(
            finished(&turn, "").unwrap(),
            "I think we should ship it on Friday."
        );
        assert!(finished(&json!({ "status": "interrupted" }), "x").is_err());
    }

    #[test]
    fn reads_account_models_and_plan_usage() {
        assert_eq!(
            read_account(
                &json!({ "account": { "type": "chatgpt", "email": "a@b.c", "planType": "pro" } })
            ),
            Account::ChatGpt {
                email: "a@b.c".into(),
                plan: "pro".into()
            }
        );
        assert_eq!(
            read_account(&json!({ "account": null })),
            Account::SignedOut
        );
        let models = read_models(&json!({ "data": [
            { "id": "gpt-6-luna", "displayName": "GPT-6-Luna", "description": "Fast and affordable model for easier tasks.",
              "hidden": false, "supportedReasoningEfforts": [{ "reasoningEffort": "low", "description": "" }],
              "defaultReasoningEffort": "medium", "isDefault": false },
            { "id": "secret", "hidden": true },
        ] }));
        assert_eq!(models.len(), 1);
        assert_eq!(
            models[0].description,
            "Fast and affordable model for easier tasks"
        );
        assert_eq!(models[0].efforts, ["low"]);
        let (limits, reached) = read_limits(&json!({ "rateLimits": {
            "primary": { "usedPercent": 67, "windowDurationMins": 10080, "resetsAt": 1794244021 },
            "secondary": null,
            "rateLimitReachedType": null,
        } }));
        assert_eq!(
            limits,
            [Limit {
                window_minutes: 10080,
                used_percent: 67.0,
                resets_at: Some(1794244021)
            }]
        );
        assert!(!reached);
    }
}
