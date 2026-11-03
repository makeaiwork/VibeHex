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
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, WrapErr, bail, eyre};

use super::local_models::{self, LocalModel};

/// The longest a model may take to load before a request gives up on it.
const LOAD_LIMIT: Duration = Duration::from_secs(120);
/// How long the server stays loaded after its last rewrite.
const IDLE_UNLOAD: Duration = Duration::from_secs(15 * 60);
const POLL: Duration = Duration::from_millis(100);
/// Room for Hex's instructions, the vocabulary, a long transcript, and its
/// rewrite.
const CONTEXT_TOKENS: &str = "8192";
const PID_FILE: &str = "server.pid";

static ENGINE: Mutex<Option<Running>> = Mutex::new(None);

struct Running {
    child: Child,
    model: &'static str,
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

/// The loaded model's server, starting it (and waiting for the model to
/// load) when it isn't running, within `deadline`.
pub fn endpoint(
    model: &'static LocalModel,
    deadline: Duration,
    cancelled: &AtomicBool,
) -> Result<Endpoint> {
    let started = Instant::now();
    let mut engine = lock_within(started, deadline, cancelled, model)?;
    if let Some(running) = engine.as_mut()
        && running.model == model.id
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
    let result = start(model).and_then(|mut running| {
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
    let running = result?;
    let endpoint = Endpoint {
        url: running.url.clone(),
        key: running.key.clone(),
    };
    *engine = Some(running);
    Ok(endpoint)
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
    static STARTED: OnceLock<()> = OnceLock::new();
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
        .ok_or_else(|| eyre!("Hex's executable has no folder"))?
        .join("llama-server");
    if !path.is_file() {
        bail!("this build of Hex has no built-in engine (llama-server)");
    }
    Ok(path)
}

fn start(model: &'static LocalModel) -> Result<Running> {
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
        pid = child.id(),
        "started the cleanup engine"
    );
    Ok(Running {
        child,
        model: model.id,
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
) -> Vec<std::ffi::OsString> {
    let mut arguments: Vec<std::ffi::OsString> = Vec::new();
    let mut push = |value: &dyn AsRef<std::ffi::OsStr>| arguments.push(value.as_ref().to_owned());
    for (flag, value) in [
        ("--model", model.as_os_str()),
        ("--host", "127.0.0.1".as_ref()),
        ("--port", port.to_string().as_ref()),
        ("--ctx-size", CONTEXT_TOKENS.as_ref()),
        ("--n-gpu-layers", "all".as_ref()),
        ("--parallel", "1".as_ref()),
        ("--alias", alias.as_ref()),
        ("--log-file", log.as_os_str()),
    ] {
        push(&flag);
        push(&value);
    }
    push(&"--no-webui");
    push(&"--offline");
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
    fn keys_are_random_and_ports_free() {
        let first = random_key().unwrap();
        assert_eq!(first.len(), 48);
        assert_ne!(first, random_key().unwrap());
        assert!(free_port().unwrap() > 0);
    }
}
