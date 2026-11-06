//! Labs › Clean up on your own server: a dictation's rewrite on OpenRouter or
//! any OpenAI-compatible server (LM Studio, Ollama, Groq, Gemini, OpenAI)
//! instead of OpenCode.
//!
//! One blocking `POST {base}/chat/completions` per dictation through Hex's
//! own HTTPS (`net`): loopback servers through the loopback-only agent, the
//! rest through the computer's proxy and trusted roots. The request runs on
//! its own thread so cancellation and the deadline end the wait at once; a
//! late answer is dropped. API keys come from the OS credential store
//! ([`keys`]), never from settings.json. Built in runs a model Hex downloads
//! ([`local_models`]) on the bundled llama.cpp server ([`engine`]).

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub mod engine;
pub(crate) mod keys;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub mod local_models;
pub mod models;

use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

use crate::app_settings::{CleanupRunsOn, CleanupServerSettings};
use crate::dictation::cleanup_pieces::Streamed;

pub const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1";
pub const LM_STUDIO_URL: &str = "http://127.0.0.1:1234/v1";
pub const OLLAMA_URL: &str = "http://127.0.0.1:11434/v1";
/// How long Test waits: a local server may still be loading its model.
pub const TEST_DEADLINE: Duration = Duration::from_secs(30);
/// Test of the built-in model may first load the model.
const ENGINE_TEST_DEADLINE: Duration = Duration::from_secs(120);
/// How often a waiting rewrite checks cancellation.
const POLL: Duration = Duration::from_millis(50);
const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
/// How much of a server's error body a message quotes.
const ERROR_EXCERPT: usize = 200;

static SAVED: RwLock<Option<CleanupServerSettings>> = RwLock::new(None);

/// Records the saved choice (`AppSettings::apply_runtime`).
pub fn set_saved(settings: &CleanupServerSettings) {
    if let Ok(mut saved) = SAVED.write() {
        *saved = Some(settings.clone());
    }
}

/// The server a dictation's rewrite runs on now: `None` means OpenCode,
/// which is also the answer while the Labs feature is off.
pub fn active() -> Option<Server> {
    if !crate::labs::ai_cleanup_server() {
        return None;
    }
    let saved = SAVED.read().ok()?.clone()?;
    Server::from_settings(&saved)
}

/// The built-in model, while it is the chosen server.
pub fn local_model() -> Option<&'static local_models::LocalModel> {
    let server = active()?;
    (server.provider == Provider::Local).then(|| local_models::chosen(&server.model))
}

/// Which credential a server uses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    OpenRouter,
    Custom,
    /// A downloaded model on the bundled engine; it needs no key of the
    /// person's.
    Local,
}

impl Provider {
    pub fn key_name(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::Custom => "custom",
            Self::Local => "local",
        }
    }
}

/// One OpenAI-compatible server and the model a rewrite asks it for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Server {
    pub provider: Provider,
    pub url: String,
    pub model: String,
}

impl Server {
    /// The saved server, or `None` for OpenCode.
    pub fn from_settings(settings: &CleanupServerSettings) -> Option<Self> {
        match settings.runs_on {
            CleanupRunsOn::Opencode => None,
            CleanupRunsOn::Openrouter => Some(Self {
                provider: Provider::OpenRouter,
                url: OPENROUTER_URL.into(),
                model: settings.openrouter_model.trim().into(),
            }),
            CleanupRunsOn::Custom => Some(Self {
                provider: Provider::Custom,
                url: settings.url.trim().into(),
                model: settings.custom_model.trim().into(),
            }),
            // The engine's address is known once it runs ([`engine::endpoint`]).
            CleanupRunsOn::Local => Some(Self {
                provider: Provider::Local,
                url: String::new(),
                model: local_models::chosen(&settings.local_model).id.into(),
            }),
        }
    }

    /// The server's name for a person: OpenRouter, LM Studio, Ollama, or its
    /// host and port.
    pub fn name(&self) -> String {
        match self.provider {
            Provider::OpenRouter => "OpenRouter".into(),
            Provider::Local => local_models::chosen(&self.model).name.into(),
            Provider::Custom => server_name(&self.url),
        }
    }
}

/// A custom server's name: LM Studio and Ollama by their usual ports on this
/// computer, otherwise the host and port.
pub fn server_name(url: &str) -> String {
    let Ok(parsed) = url::Url::parse(url.trim()) else {
        return "your server".into();
    };
    let host = parsed.host_str().unwrap_or("your server");
    let port = parsed.port_or_known_default();
    if is_loopback(&parsed) {
        match port {
            Some(1234) => return "LM Studio".into(),
            Some(11434) => return "Ollama".into(),
            _ => {}
        }
    }
    match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.into(),
    }
}

/// Rewrites one transcript: `instructions` as the system message, the
/// transcript block as the user's. The key is read from the credential store.
pub fn generate(
    server: &Server,
    instructions: &str,
    transcript: &str,
    deadline: Duration,
    cancelled: &AtomicBool,
    streamed: Option<Streamed>,
) -> Result<String> {
    if server.provider == Provider::Local {
        return generate_locally(
            server,
            engine::CONTEXT_TOKENS,
            instructions,
            transcript,
            deadline,
            cancelled,
            streamed,
        );
    }
    let key = keys::get(server.provider);
    generate_with_key(
        server,
        key.as_deref(),
        instructions,
        transcript,
        deadline,
        cancelled,
        streamed,
    )
}

/// Rewrites with the built-in model: starts the engine with the chosen model when it
/// isn't running (loading counts against `deadline`), then asks it.
fn generate_locally(
    server: &Server,
    context: u32,
    instructions: &str,
    transcript: &str,
    deadline: Duration,
    cancelled: &AtomicBool,
    streamed: Option<Streamed>,
) -> Result<String> {
    let started = Instant::now();
    let model = local_models::chosen(&server.model);
    let endpoint = engine::endpoint_with(model, context, deadline, cancelled)?;
    let running = Server {
        url: endpoint.url,
        ..server.clone()
    };
    generate_with_key(
        &running,
        Some(&endpoint.key),
        instructions,
        transcript,
        deadline.saturating_sub(started.elapsed()),
        cancelled,
        streamed,
    )
}

/// Labs › Files: [`generate`] on the built-in engine in a window of at least
/// `context` tokens (another server ignores it), its answer streamed to
/// `streamed` as it arrives.
pub fn generate_in_window(
    server: &Server,
    context: u32,
    instructions: &str,
    transcript: &str,
    deadline: Duration,
    cancelled: &AtomicBool,
    streamed: Option<Streamed>,
) -> Result<String> {
    if server.provider != Provider::Local {
        return generate(
            server,
            instructions,
            transcript,
            deadline,
            cancelled,
            streamed,
        );
    }
    generate_locally(
        server,
        context,
        instructions,
        transcript,
        deadline,
        cancelled,
        streamed,
    )
}

/// How many tokens the built-in `model` reads `text` as, from its server's
/// `/tokenize` (started at the usual window if it isn't running).
pub fn count_tokens(
    model: &'static local_models::LocalModel,
    text: &str,
    deadline: Duration,
    cancelled: &AtomicBool,
) -> Result<usize> {
    #[derive(Serialize)]
    struct Tokenize<'a> {
        content: &'a str,
    }
    #[derive(Deserialize)]
    struct Tokens {
        tokens: Vec<serde_json::Value>,
    }
    // Any window will do: a wide one already running is kept.
    let endpoint = engine::endpoint_with(model, engine::CONTEXT_TOKENS, deadline, cancelled)?;
    let base = api_base(&endpoint.url);
    let url = format!("{}/tokenize", base.strip_suffix("/v1").unwrap_or(base));
    let exchange = Exchange {
        loopback: true,
        url,
        key: Some(endpoint.key),
        openrouter: false,
        body: Some(serde_json::to_vec(&Tokenize { content: text })?),
        timeout: deadline,
        limit: MAX_RESPONSE_BYTES * 4,
        name: model.name.into(),
        streamed: None,
    };
    let body = exchange.run()?;
    let tokens: Tokens = serde_json::from_slice(&body)
        .map_err(|error| eyre!("{} sent token counts Hex can't read: {error}", model.name))?;
    Ok(tokens.tokens.len())
}

/// Sends a one-word request and reports how long the answer took.
pub fn test(server: &Server, key: Option<&str>) -> Result<Duration> {
    let started = Instant::now();
    if server.provider == Provider::Local {
        generate_locally(
            server,
            engine::CONTEXT_TOKENS,
            "Reply with the single word OK.",
            "ping",
            ENGINE_TEST_DEADLINE,
            &AtomicBool::new(false),
            None,
        )?;
        return Ok(started.elapsed());
    }
    generate_with_key(
        server,
        key,
        "Reply with the single word OK.",
        "ping",
        TEST_DEADLINE,
        &AtomicBool::new(false),
        None,
    )?;
    Ok(started.elapsed())
}

fn generate_with_key(
    server: &Server,
    key: Option<&str>,
    instructions: &str,
    transcript: &str,
    deadline: Duration,
    cancelled: &AtomicBool,
    streamed: Option<Streamed>,
) -> Result<String> {
    let started = Instant::now();
    let name = server.name();
    if server.url.is_empty() {
        return Err(eyre!("{name} has no URL yet"));
    }
    if server.model.is_empty() {
        return Err(eyre!("choose a model for {name}"));
    }
    let key = key.map(str::trim).filter(|key| !key.is_empty());
    if server.provider == Provider::OpenRouter && key.is_none() {
        return Err(eyre!("add your OpenRouter API key"));
    }
    let url = endpoint(&server.url);
    let parsed = url::Url::parse(&url).map_err(|_| eyre!("{name}'s URL isn't valid: {url}"))?;
    let body = serde_json::to_vec(&ChatRequest {
        model: &server.model,
        messages: [
            Message {
                role: "system",
                content: instructions,
            },
            Message {
                role: "user",
                content: transcript,
            },
        ],
        temperature: 0.0,
        // Streamed only for Labs › Cleanup progress, which shows the
        // answer arriving; a server that ignores it answers whole.
        stream: streamed.is_some(),
    })?;
    let exchange = Exchange {
        loopback: is_loopback(&parsed),
        url,
        key: key.map(str::to_owned),
        openrouter: server.provider == Provider::OpenRouter,
        body: Some(body),
        timeout: deadline,
        limit: MAX_RESPONSE_BYTES,
        name: name.clone(),
        streamed,
    };
    let (sender, receiver) = mpsc::channel();
    let answered = name.clone();
    thread::Builder::new()
        .name("hex-cleanup-server".into())
        .spawn(move || {
            let _ = sender.send(exchange.run().and_then(|body| answer(&answered, &body)));
        })
        .map_err(|error| eyre!("couldn't start the request to {name}: {error}"))?;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(eyre!("the request to {name} was cancelled"));
        }
        let remaining = deadline.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err(eyre!(
                "{name} didn't answer within {} seconds",
                deadline.as_secs()
            ));
        }
        match receiver.recv_timeout(remaining.min(POLL)) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(eyre!("the request to {name} stopped"));
            }
        }
    }
}

/// `{base}/chat/completions`, whether or not the base ends in a slash or
/// already names the endpoint.
fn endpoint(base: &str) -> String {
    format!("{}/chat/completions", api_base(base))
}

/// The base URL without a trailing slash or a named endpoint.
fn api_base(base: &str) -> &str {
    let base = base.trim().trim_end_matches('/');
    base.strip_suffix("/chat/completions").unwrap_or(base)
}

fn is_loopback(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [Message<'a>; 2],
    temperature: f32,
    stream: bool,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
}

/// One request to a server: a POST with `body`, else a GET.
struct Exchange {
    url: String,
    loopback: bool,
    key: Option<String>,
    openrouter: bool,
    body: Option<Vec<u8>>,
    timeout: Duration,
    /// The most of the answer's body read.
    limit: u64,
    name: String,
    /// Reads a streamed answer as it arrives, reporting its length and text.
    streamed: Option<Streamed>,
}

impl Exchange {
    /// The answer's body, or why the server refused or couldn't be reached.
    fn run(self) -> Result<Vec<u8>> {
        let timeout = self.timeout;
        let configure = |config: crate::net::Config| {
            config
                .http_status_as_error(false)
                .timeout_global(Some(timeout))
        };
        let agent = if self.loopback {
            crate::net::loopback_agent(configure)
        } else {
            crate::net::agent(&self.url, configure)
        };
        let mut builder = ureq::http::Request::builder()
            .method(if self.body.is_some() { "POST" } else { "GET" })
            .uri(self.url.as_str());
        if self.body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }
        if let Some(key) = &self.key {
            builder = builder.header("Authorization", format!("Bearer {key}"));
        }
        if self.openrouter {
            builder = builder.header("X-Title", "Hex");
        }
        let invalid = |error: ureq::http::Error| eyre!("invalid request to {}: {error}", self.name);
        let sent = match self.body {
            Some(body) => agent.run(builder.body(body).map_err(invalid)?),
            None => agent.run(builder.body(()).map_err(invalid)?),
        };
        let mut response =
            sent.map_err(|error| eyre!(crate::net::unreachable(&self.url, &error)))?;
        let status = response.status().as_u16();
        let event_stream = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("text/event-stream"));
        if let Some(streamed) = self.streamed.as_ref().filter(|_| event_stream)
            && (200..300).contains(&status)
        {
            let reader = response.body_mut().with_config().limit(self.limit).reader();
            let text = read_events(std::io::BufReader::new(reader), streamed.as_ref())
                .map_err(|error| eyre!("{} stopped answering: {error}", self.name))?;
            // The whole answer, as a non-streamed one reads.
            return Ok(serde_json::to_vec(&serde_json::json!({
                "choices": [{ "message": { "content": text } }]
            }))?);
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(self.limit)
            .read_to_vec()
            .map_err(|error| eyre!(crate::net::unreachable(&self.url, &error)))?;
        if !(200..300).contains(&status) {
            return Err(eyre!(refusal(&self.name, status, &body)));
        }
        Ok(body)
    }
}

/// A streamed chat answer (server-sent events) read to its end: each
/// event's `choices[0].delta.content` appended, with `on` told how many
/// characters have arrived and what they say.
fn read_events(
    reader: impl std::io::BufRead,
    on: &(dyn Fn(usize, &str) + Send + Sync),
) -> std::io::Result<String> {
    let mut text = String::new();
    let mut chars = 0;
    for line in reader.lines() {
        let line = line?;
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data == "[DONE]" {
            break;
        }
        let Ok(event) = serde_json::from_str::<StreamEvent>(data) else {
            continue;
        };
        if let Some(piece) = event
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.delta.content)
        {
            chars += piece.chars().count();
            text.push_str(&piece);
            on(chars, &text);
        }
    }
    Ok(text)
}

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(default)]
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
}

#[derive(Default, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}

/// Why a server refused, with the reason it gave when it gave one.
fn refusal(name: &str, status: u16, body: &[u8]) -> String {
    let said = server_message(body)
        .map(|message| format!(": {message}"))
        .unwrap_or_default();
    match status {
        401 | 403 => format!("{name} refused the API key (HTTP {status}){said}"),
        404 => format!("{name} has no such model or URL (HTTP 404){said}"),
        _ => format!("{name} answered HTTP {status}{said}"),
    }
}

/// `{"error": {"message": …}}`, `{"error": "…"}`, or `{"message": …}`, cut short.
fn server_message(body: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let message = value
        .pointer("/error/message")
        .or_else(|| value.get("error"))
        .or_else(|| value.get("message"))?
        .as_str()?
        .trim();
    if message.is_empty() {
        return None;
    }
    Some(match message.char_indices().nth(ERROR_EXCERPT) {
        Some((cut, _)) => format!("{}…", &message[..cut]),
        None => message.into(),
    })
}

fn answer(name: &str, body: &[u8]) -> Result<String> {
    let response: ChatResponse = serde_json::from_slice(body)
        .map_err(|error| eyre!("{name} sent an answer Hex can't read: {error}"))?;
    let content = response
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .unwrap_or_default();
    let text = clean(&content);
    if text.is_empty() {
        Err(eyre!("{name} returned empty text"))
    } else {
        Ok(text)
    }
}

/// Where a reasoning model's final answer starts when its server leaves the
/// reasoning in the answer: Gemma 4 (`…thought<channel|>answer`, the opening
/// marker already stripped by LM Studio) and gpt-oss's final channel.
const FINAL_MARKERS: [&str; 2] = ["<|channel|>final<|message|>", "<channel|>"];

/// The paste text from a model's answer: without its reasoning
/// (`<think>…</think>`, [`FINAL_MARKERS`]) or a Markdown fence around the
/// whole answer.
fn clean(content: &str) -> String {
    let mut text = content.trim();
    if let Some(after) = FINAL_MARKERS
        .iter()
        .find_map(|marker| text.rsplit_once(marker).map(|(_, after)| after))
    {
        text = after.trim();
    }
    if let Some(rest) = text.strip_prefix("<think>") {
        text = rest.split_once("</think>").map_or("", |(_, after)| after);
    } else if let Some((_, after)) = text.split_once("</think>") {
        text = after;
    }
    let text = text.trim();
    let unfenced = text
        .strip_prefix("```")
        .and_then(|rest| rest.strip_suffix("```"))
        .map(|inner| {
            // Drop a language tag on the opening fence's line.
            match inner.split_once('\n') {
                Some((tag, body)) if !tag.trim().contains(' ') => body,
                _ => inner,
            }
        });
    unfenced.unwrap_or(text).trim().into()
}

#[cfg(test)]
mod tests;
