//! The bundled `llama-server` (llama.cpp, `Contents/MacOS/llama-server`)
//! that runs a downloaded cleanup model ([`super::local_models`]) for
//! Labs › Clean up on your own server › Built in.
//!
//! One server at a time, started by Hex on demand: listening on 127.0.0.1 at
//! a free port, with a random API key passed through its environment (never
//! argv), offline, without its web UI, logging to `logs/llama-server.log`.
//! A dictation's press warms it ([`start_watcher`]), so the model is loaded by
//! the time the transcript needs it; 15 minutes without a rewrite unload it,
//! and quitting Hex stops it. Its PID is kept in `cleanup-models/server.pid`
//! so a server a crashed Hex left behind is stopped before the next one
//! starts. The rewrite itself is the same OpenAI-compatible request the other
//! servers get.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, WrapErr, bail, eyre};

use super::local_models::{self, LocalModel};

/// The longest a model may take to load before a request gives up on it.
const LOAD_LIMIT: Duration = Duration::from_secs(120);
/// How long the server stays loaded after its last rewrite.
const IDLE_UNLOAD: Duration = Duration::from_secs(15 * 60);
const POLL: Duration = Duration::from_millis(100);
/// Room for Hex's instructions, the vocabulary, a long transcript, and its
/// rewrite: the window dictation runs in.
pub const CONTEXT_TOKENS: u32 = 8192;
/// The wide window Labs › Files asks for when a whole text doesn't fit the
/// usual one; its cache is kept at 8 bits to halve its memory.
pub const WIDE_CONTEXT_TOKENS: u32 = 32_768;
const PID_FILE: &str = "server.pid";

static ENGINE: Mutex<Option<Running>> = Mutex::new(None);

struct Running {
    child: Child,
    model: &'static str,
    /// The server's context window, in tokens.
    context: u32,
    url: String,
    key: String,
    last_used: Instant,
}

/// Where a loaded model answers.
#[derive(Clone, Debug)]
pub struct Endpoint {
    /// The OpenAI-compatible base, `http://127.0.0.1:<port>/v1`.
    pub url: String,
    pub key: String,
}

/// What the engine is doing, for Output's status line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Stopped,
    Loading,
    Ready,
}

static LOADING: AtomicBool = AtomicBool::new(false);
/// While a model loads: which, when it started, and how long it should take.
static LOAD: Mutex<Option<(&'static str, Instant, Duration)>> = Mutex::new(None);

/// The model now loading and how far it is, from 0 to 1: an estimate, since
/// llama-server doesn't say, from how long this model took last time (from
/// its size the first time), never past 99% until it answers. `None` while
/// nothing loads.
pub fn load_progress() -> Option<(&'static str, f32)> {
    let (model, started, expected) = (*LOAD.lock().unwrap_or_else(|error| error.into_inner()))?;
    Some((model, estimate(started.elapsed(), expected)))
}

/// Linear to 90% at `expected`, then creeping towards 99%.
fn estimate(elapsed: Duration, expected: Duration) -> f32 {
    let t = elapsed.as_secs_f32() / expected.as_secs_f32().max(0.1);
    if t <= 1.0 {
        0.9 * t
    } else {
        0.99 - 0.09 * (1.0 - t).exp()
    }
}

/// How long `model` should take to load: its last load, else about a
/// second plus its file read at 1.5 GB/s.
fn expected_load(model: &LocalModel) -> Duration {
    load_time_file(model)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|millis| millis.trim().parse().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs_f64(1.0 + model.bytes as f64 / 1.5e9))
}

fn load_time_file(model: &LocalModel) -> Option<std::path::PathBuf> {
    local_models::dir()
        .ok()
        .map(|dir| dir.join(format!("{}.load-ms", model.id)))
}

/// The loaded model's server, starting it (and waiting for the model to
/// load) when it isn't running, within `deadline`.
pub fn endpoint(
    model: &'static LocalModel,
    deadline: Duration,
    cancelled: &AtomicBool,
) -> Result<Endpoint> {
    endpoint_with(model, CONTEXT_TOKENS, deadline, cancelled)
}

/// [`endpoint`] with a window of at least `context` tokens: a server already
/// running `model` with one that wide is kept, else it restarts with
/// `context`.
pub fn endpoint_with(
    model: &'static LocalModel,
    context: u32,
    deadline: Duration,
    cancelled: &AtomicBool,
) -> Result<Endpoint> {
    let started = Instant::now();
    let mut engine = lock_within(started, deadline, cancelled, model)?;
    if let Some(running) = engine.as_mut()
        && running.model == model.id
        && running.context >= context
        && matches!(running.child.try_wait(), Ok(None))
    {
        running.last_used = Instant::now();
        return Ok(Endpoint {
            url: running.url.clone(),
            key: running.key.clone(),
        });
    }
    stop_locked(&mut engine);
    if !model.installed() {
        bail!("{} isn't downloaded yet; download it in Output", model.name);
    }
    LOADING.store(true, Ordering::Release);
    let loading = Instant::now();
    *LOAD.lock().unwrap_or_else(|error| error.into_inner()) =
        Some((model.id, loading, expected_load(model)));
    let result = start(model, context).and_then(|mut running| {
        let remaining = deadline.saturating_sub(started.elapsed()).min(LOAD_LIMIT);
        match wait_ready(&mut running, remaining, cancelled) {
            Ok(()) => Ok(running),
            Err(error) => {
                stop_child(&mut running.child);
                Err(error)
            }
        }
    });
    LOADING.store(false, Ordering::Release);
    *LOAD.lock().unwrap_or_else(|error| error.into_inner()) = None;
    let running = result?;
    if let Some(path) = load_time_file(model) {
        let _ = fs::write(path, loading.elapsed().as_millis().to_string());
    }
    let endpoint = Endpoint {
        url: running.url.clone(),
        key: running.key.clone(),
    };
    *engine = Some(running);
    Ok(endpoint)
}

/// The running server's window, in tokens.
pub fn running_context() -> Option<u32> {
    let mut engine = ENGINE.try_lock().ok()?;
    let running = engine.as_mut()?;
    matches!(running.child.try_wait(), Ok(None)).then_some(running.context)
}

/// Back to dictation's window once Labs › Files is done with a wide one:
/// stops a server running wider and loads its model again at the usual
/// size, in the background.
pub fn narrow() {
    let model = {
        let mut engine = ENGINE.lock().unwrap_or_else(|error| error.into_inner());
        let Some(running) = engine.as_ref() else {
            return;
        };
        if running.context <= CONTEXT_TOKENS {
            return;
        }
        let model = running.model;
        stop_locked(&mut engine);
        model
    };
    if let Some(model) = local_models::find(model) {
        warm(model);
    }
}

/// Starts loading `model` in the background unless it already runs or loads.
pub fn warm(model: &'static LocalModel) {
    if !model.installed() || LOADING.load(Ordering::Acquire) || running_model() == Some(model.id) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("hex-cleanup-engine-warm".into())
        .spawn(move || {
            if let Err(error) = endpoint(model, LOAD_LIMIT, &AtomicBool::new(false)) {
                tracing::warn!(%error, model = model.id, "the cleanup model didn't load");
            }
        });
}

pub fn state() -> State {
    if LOADING.load(Ordering::Acquire) {
        return State::Loading;
    }
    match running_model() {
        Some(_) => State::Ready,
        None => State::Stopped,
    }
}

/// The model the running server holds.
pub fn running_model() -> Option<&'static str> {
    let mut engine = ENGINE.try_lock().ok()?;
    let running = engine.as_mut()?;
    matches!(running.child.try_wait(), Ok(None)).then_some(running.model)
}

/// Stops the server (quitting Hex).
pub fn stop() {
    let mut engine = ENGINE.lock().unwrap_or_else(|error| error.into_inner());
    stop_locked(&mut engine);
}

/// Stops the server if it runs `model` (deleting the model).
pub fn stop_model(model: &str) {
    let mut engine = ENGINE.lock().unwrap_or_else(|error| error.into_inner());
    if engine
        .as_ref()
        .is_some_and(|running| running.model == model)
    {
        stop_locked(&mut engine);
    }
}

/// Watches for a dictation's press while Built in is chosen, to warm the
/// chosen model, and unloads it after [`IDLE_UNLOAD`] without a rewrite.
/// Started once by the app.
#[cfg(target_os = "macos")]
pub fn start_watcher() {
    static STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("hex-cleanup-engine".into())
        .spawn(|| {
            let mut was_busy = false;
            loop {
                std::thread::sleep(Duration::from_millis(250));
                if crate::desktop::process_exit::quitting() {
                    return;
                }
                let busy = dictation_busy();
                if busy
                    && !was_busy
                    && let Some(model) = super::local_model()
                {
                    warm(model);
                }
                was_busy = busy;
                unload_if_idle();
            }
        });
}

#[cfg(target_os = "macos")]
fn dictation_busy() -> bool {
    crate::parakeet::dictation_busy()
}

#[cfg(not(target_os = "macos"))]
fn dictation_busy() -> bool {
    false
}

fn unload_if_idle() {
    let Ok(mut engine) = ENGINE.try_lock() else {
        return;
    };
    let idle = engine
        .as_ref()
        .is_some_and(|running| running.last_used.elapsed() > IDLE_UNLOAD);
    if idle && !dictation_busy() {
        tracing::info!("unloading the idle cleanup model");
        stop_locked(&mut engine);
    }
}

/// Waits for the engine's lock: another request may be loading the model.
fn lock_within(
    started: Instant,
    deadline: Duration,
    cancelled: &AtomicBool,
    model: &LocalModel,
) -> Result<MutexGuard<'static, Option<Running>>> {
    loop {
        match ENGINE.try_lock() {
            Ok(engine) => return Ok(engine),
            Err(TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
            Err(TryLockError::WouldBlock) => {}
        }
        if cancelled.load(Ordering::Acquire) {
            bail!("the request to {} was cancelled", model.name);
        }
        if started.elapsed() >= deadline {
            bail!("{} is still loading", model.name);
        }
        std::thread::sleep(POLL);
    }
}

fn stop_locked(engine: &mut Option<Running>) {
    if let Some(mut running) = engine.take() {
        stop_child(&mut running.child);
        if let Ok(dir) = local_models::dir() {
            let _ = fs::remove_file(dir.join(PID_FILE));
        }
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// The bundled server beside Hex's executable (`HEX_LLAMA_SERVER` names
/// another, for development).
fn binary() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("HEX_LLAMA_SERVER") {
        return Ok(PathBuf::from(path));
    }
    let executable = std::env::current_exe()?.canonicalize()?;
    let path = executable
        .parent()
        .ok_or_else(|| eyre!("the VibeHex executable has no folder"))?
        .join("llama-server");
    if !path.is_file() {
        bail!("this build of VibeHex has no built-in engine (llama-server)");
    }
    Ok(path)
}

fn start(model: &'static LocalModel, context: u32) -> Result<Running> {
    let binary = binary()?;
    stop_orphan();
    let port = free_port()?;
    let key = random_key()?;
    let log = crate::app_paths::logs_dir()?;
    fs::create_dir_all(&log)?;
    let mut command = crate::desktop::background_process::command(&binary);
    command
        .args(arguments(
            &model.path()?,
            port,
            &log.join("llama-server.log"),
            model.id,
            context,
        ))
        .env("LLAMA_API_KEY", &key)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = command
        .spawn()
        .wrap_err_with(|| format!("couldn't start {}", binary.display()))?;
    if let Ok(dir) = local_models::dir() {
        let _ = fs::write(dir.join(PID_FILE), child.id().to_string());
    }
    tracing::info!(
        model = model.id,
        port,
        context,
        pid = child.id(),
        "started the cleanup engine"
    );
    Ok(Running {
        child,
        model: model.id,
        context,
        url: format!("http://127.0.0.1:{port}/v1"),
        key,
        last_used: Instant::now(),
    })
}

fn arguments(
    model: &std::path::Path,
    port: u16,
    log: &std::path::Path,
    alias: &str,
    context: u32,
) -> Vec<std::ffi::OsString> {
    let mut arguments: Vec<std::ffi::OsString> = Vec::new();
    let mut push = |value: &dyn AsRef<std::ffi::OsStr>| arguments.push(value.as_ref().to_owned());
    for (flag, value) in [
        ("--model", model.as_os_str()),
        ("--host", "127.0.0.1".as_ref()),
        ("--port", port.to_string().as_ref()),
        ("--ctx-size", context.to_string().as_ref()),
        ("--n-gpu-layers", "all".as_ref()),
        ("--parallel", "1".as_ref()),
        ("--alias", alias.as_ref()),
        ("--log-file", log.as_os_str()),
        // Cleanup wants the answer, not a chain of thought: models that can
        // think (Gemma 4) answer straight away, faster and within the deadline.
        ("--reasoning", "off".as_ref()),
    ] {
        push(&flag);
        push(&value);
    }
    push(&"--no-webui");
    push(&"--offline");
    // A wide window's cache at 8 bits takes half the memory; quantizing the
    // values needs flash attention.
    if context > CONTEXT_TOKENS {
        for argument in [
            "--flash-attn",
            "on",
            "--cache-type-k",
            "q8_0",
            "--cache-type-v",
            "q8_0",
        ] {
            push(&argument);
        }
    }
    arguments
}

/// Polls `/health` until the model has loaded (200; 503 while loading).
fn wait_ready(running: &mut Running, limit: Duration, cancelled: &AtomicBool) -> Result<()> {
    let started = Instant::now();
    let health = format!("{}/health", running.url.trim_end_matches("/v1"));
    let agent = crate::net::loopback_agent(|config| {
        config
            .timeout_global(Some(Duration::from_secs(2)))
            .http_status_as_error(false)
    });
    loop {
        if let Ok(Some(status)) = running.child.try_wait() {
            bail!(
                "the built-in engine stopped while loading ({status}); see logs/llama-server.log"
            );
        }
        if let Ok(response) = agent.get(&health).call()
            && response.status() == 200
        {
            tracing::info!(elapsed = ?started.elapsed(), "the cleanup model is loaded");
            return Ok(());
        }
        if cancelled.load(Ordering::Acquire) {
            bail!("loading the cleanup model was cancelled");
        }
        if started.elapsed() >= limit {
            bail!(
                "the cleanup model didn't load within {} seconds",
                limit.as_secs()
            );
        }
        std::thread::sleep(POLL);
    }
}

fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn random_key() -> Result<String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(|error| eyre!("no secure random source: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Stops a server an earlier Hex started and couldn't stop (a crash), when
/// its PID still names a `llama-server`.
fn stop_orphan() {
    let Ok(dir) = local_models::dir() else {
        return;
    };
    let file = dir.join(PID_FILE);
    let Some(pid) = fs::read_to_string(&file)
        .ok()
        .and_then(|text| text.trim().parse::<i32>().ok())
    else {
        return;
    };
    let _ = fs::remove_file(&file);
    #[cfg(unix)]
    {
        let name = crate::desktop::background_process::command("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
        if pid > 1 && name.ends_with("llama-server") {
            tracing::info!(pid, "stopping a cleanup engine left from an earlier run");
            // SAFETY: kill only signals the process; the PID was checked to
            // be a llama-server just now.
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_listens_on_loopback_offline_without_its_web_ui() {
        let arguments: Vec<String> = arguments(
            std::path::Path::new("/models/m.gguf"),
            4242,
            std::path::Path::new("/logs/l.log"),
            "m",
            CONTEXT_TOKENS,
        )
        .into_iter()
        .map(|argument| argument.into_string().unwrap())
        .collect();
        let pairs: Vec<(&str, &str)> = arguments
            .chunks(2)
            .filter_map(|pair| match pair {
                [flag, value] if !value.starts_with("--") => Some((flag.as_str(), value.as_str())),
                _ => None,
            })
            .collect();
        assert!(pairs.contains(&("--host", "127.0.0.1")));
        assert!(pairs.contains(&("--port", "4242")));
        assert!(pairs.contains(&("--model", "/models/m.gguf")));
        assert!(pairs.contains(&("--reasoning", "off")));
        assert!(arguments.iter().any(|argument| argument == "--no-webui"));
        assert!(arguments.iter().any(|argument| argument == "--offline"));
        // The key travels in the environment, never in argv.
        assert!(
            !arguments
                .iter()
                .any(|argument| argument.contains("api-key"))
        );
    }

    #[test]
    fn a_load_estimate_reaches_ninety_percent_on_time_and_never_finishes() {
        let expected = Duration::from_secs(2);
        assert_eq!(estimate(Duration::ZERO, expected), 0.0);
        assert!((estimate(Duration::from_secs(1), expected) - 0.45).abs() < 1e-4);
        assert!((estimate(expected, expected) - 0.9).abs() < 1e-4);
        let late = estimate(Duration::from_secs(8), expected);
        assert!(late > 0.95 && late < 0.99, "{late}");
    }

    #[test]
    fn keys_are_random_and_ports_free() {
        let first = random_key().unwrap();
        assert_eq!(first.len(), 48);
        assert_ne!(first, random_key().unwrap());
        assert!(free_port().unwrap() > 0);
    }
}
