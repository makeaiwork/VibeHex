use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

/// A one-request server on loopback: answers `status` and `body`, and
/// hands back the request it read.
fn serve_once(status: u16, body: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut payload = vec![0; length];
        reader.read_exact(&mut payload).unwrap();
        let _ = sender.send(format!("{head}{}", String::from_utf8_lossy(&payload)));
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    (url, receiver)
}

fn custom(url: &str) -> Server {
    Server {
        provider: Provider::Custom,
        url: url.into(),
        model: "gemma-4-e4b-it-mlx".into(),
        effort: String::new(),
    }
}

fn run(server: &Server, key: Option<&str>) -> Result<String> {
    generate_with_key(
        server,
        key,
        "Spell Claude Code in English.",
        "<transcript>\nклод код\n</transcript>",
        Duration::from_secs(5),
        &AtomicBool::new(false),
        None,
    )
}

#[test]
fn a_rewrite_sends_instructions_and_transcript_as_chat_messages() {
    let (url, request) = serve_once(
        200,
        r#"{"choices":[{"message":{"role":"assistant","content":"  Claude Code  "}}]}"#,
    );
    let text = run(&custom(&url), Some("lm-key")).unwrap();
    assert_eq!(text, "Claude Code");
    let request = request.recv().unwrap();
    assert!(
        request.starts_with("POST /v1/chat/completions "),
        "{request}"
    );
    assert!(
        request.contains("authorization: Bearer lm-key")
            || request.contains("Authorization: Bearer lm-key"),
        "{request}"
    );
    let body: serde_json::Value =
        serde_json::from_str(&request[request.find("\r\n\r\n").unwrap() + 4..]).unwrap();
    assert_eq!(body["model"], "gemma-4-e4b-it-mlx");
    assert_eq!(body["stream"], false);
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(
        body["messages"][0]["content"],
        "Spell Claude Code in English."
    );
    assert_eq!(body["messages"][1]["role"], "user");
    assert_eq!(
        body["messages"][1]["content"],
        "<transcript>\nклод код\n</transcript>"
    );
}

#[test]
fn a_local_server_needs_no_key() {
    let (url, request) = serve_once(200, r#"{"choices":[{"message":{"content":"MCP"}}]}"#);
    assert_eq!(run(&custom(&url), None).unwrap(), "MCP");
    assert!(
        !request
            .recv()
            .unwrap()
            .to_ascii_lowercase()
            .contains("authorization")
    );
}

#[test]
fn refusals_name_the_server_and_its_reason() {
    let (url, _request) = serve_once(401, r#"{"error":{"message":"No auth credentials found"}}"#);
    let error = run(&custom(&url), Some("wrong")).unwrap_err().to_string();
    assert!(error.contains("refused the API key (HTTP 401)"), "{error}");
    assert!(error.contains("No auth credentials found"), "{error}");

    let (url, _request) = serve_once(404, r#"{"error":"model 'gemma' not found"}"#);
    let error = run(&custom(&url), None).unwrap_err().to_string();
    assert!(error.contains("no such model or URL (HTTP 404)"), "{error}");
    assert!(error.contains("model 'gemma' not found"), "{error}");
}

#[test]
fn an_empty_answer_is_an_error_so_the_transcript_pastes() {
    let (url, _request) = serve_once(
        200,
        r#"{"choices":[{"message":{"content":"<think>hmm</think>  "}}]}"#,
    );
    let error = run(&custom(&url), None).unwrap_err().to_string();
    assert!(error.contains("returned empty text"), "{error}");
}

#[test]
fn a_silent_server_is_bounded_by_the_deadline_and_cancellation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let _hold = thread::spawn(move || {
        let held: Vec<_> = listener.incoming().take(2).collect();
        thread::sleep(Duration::from_secs(10));
        drop(held);
    });
    let started = Instant::now();
    let error = generate_with_key(
        &custom(&url),
        None,
        "x",
        "y",
        Duration::from_millis(400),
        &AtomicBool::new(false),
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("didn't answer within"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(3));

    let started = Instant::now();
    let error = generate_with_key(
        &custom(&url),
        None,
        "x",
        "y",
        Duration::from_secs(30),
        &AtomicBool::new(true),
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("was cancelled"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn missing_settings_fail_before_any_request() {
    let mut server = custom("http://127.0.0.1:9/v1");
    server.model.clear();
    assert!(
        run(&server, None)
            .unwrap_err()
            .to_string()
            .contains("choose a model")
    );
    let openrouter = Server::from_settings(&CleanupServerSettings {
        runs_on: CleanupRunsOn::Openrouter,
        openrouter_model: "google/gemini-2.5-flash-lite".into(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(openrouter.url, OPENROUTER_URL);
    let error = run(&openrouter, None).unwrap_err().to_string();
    assert!(error.contains("OpenRouter API key"), "{error}");
}

#[test]
fn opencode_and_the_lab_off_choose_no_server() {
    assert_eq!(
        Server::from_settings(&CleanupServerSettings::default()),
        None
    );
    set_saved(&CleanupServerSettings {
        runs_on: CleanupRunsOn::Custom,
        url: LM_STUDIO_URL.into(),
        custom_model: "gemma".into(),
        ..Default::default()
    });
    // Labs › Clean up on your own server is off by default. Set the switch
    // here: unset, it reads this computer's saved settings.
    crate::labs::apply(&crate::app_settings::LabsSettings::default());
    assert_eq!(active(), None);
    crate::labs::apply(&crate::app_settings::LabsSettings {
        ai_cleanup_server: true,
        ..Default::default()
    });
    assert_eq!(active().map(|server| server.model), Some("gemma".into()));
    crate::labs::apply(&crate::app_settings::LabsSettings::default());
}

#[test]
fn endpoints_and_names() {
    assert_eq!(
        endpoint("http://127.0.0.1:1234/v1/"),
        "http://127.0.0.1:1234/v1/chat/completions"
    );
    assert_eq!(
        endpoint(" http://127.0.0.1:1234/v1 "),
        "http://127.0.0.1:1234/v1/chat/completions"
    );
    assert_eq!(
        endpoint("https://api.groq.com/openai/v1/chat/completions"),
        "https://api.groq.com/openai/v1/chat/completions"
    );
    assert_eq!(server_name(LM_STUDIO_URL), "LM Studio");
    assert_eq!(server_name("http://localhost:11434/v1"), "Ollama");
    assert_eq!(
        server_name("http://192.168.1.5:1234/v1"),
        "192.168.1.5:1234"
    );
    assert_eq!(
        server_name("https://api.groq.com/openai/v1"),
        "api.groq.com"
    );
}

#[test]
fn answers_lose_reasoning_and_fences() {
    assert_eq!(
        clean("<think>\nthe user said\n</think>\n\nClaude Code"),
        "Claude Code"
    );
    assert_eq!(clean("reasoning</think>GitHub"), "GitHub");
    assert_eq!(
        clean("The user said \"клод код\".\n\nI will use it.<channel|>Claude Code"),
        "Claude Code"
    );
    assert_eq!(
        clean(
            "<|channel|>analysis<|message|>hmm<|end|><|start|>assistant<|channel|>final<|message|>MCP"
        ),
        "MCP"
    );
    assert_eq!(clean("```text\nMCP server\n```"), "MCP server");
    assert_eq!(clean("```\nMCP server\n```"), "MCP server");
    assert_eq!(clean("Run `todo.md` now"), "Run `todo.md` now");
}

#[test]
fn the_model_list_is_a_get_of_models_with_the_key() {
    let (url, request) = serve_once(
        200,
        r#"{"data":[{"id":"gemma-4-e4b-it-mlx@4bit"},{"id":"text-embedding-nomic"}]}"#,
    );
    let listed = models::list(&custom(&format!("{url}/")), Some(" sk-local ")).unwrap();
    assert_eq!(
        listed,
        [models::ServerModel {
            id: "gemma-4-e4b-it-mlx@4bit".into(),
            name: String::new()
        }]
    );
    let request = request.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(request.starts_with("GET /v1/models HTTP/1.1"), "{request}");
    assert!(
        request.contains("authorization: Bearer sk-local")
            || request.contains("Authorization: Bearer sk-local"),
        "{request}"
    );

    let (url, _) = serve_once(401, r#"{"error":{"message":"bad key"}}"#);
    let why = models::list(&custom(&url), None).unwrap_err().to_string();
    assert!(
        why.contains("refused the API key") && why.contains("bad key"),
        "{why}"
    );
}

/// A streamed answer reads whole, its length and text so far reported as
/// it arrives.
#[test]
fn a_streamed_answer_reads_whole_and_reports_its_length() {
    let events = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
                  data: {\"choices\":[{\"delta\":{\"content\":\"Привет, \"}}]}\n\n\
                  : keep-alive\n\n\
                  data: {\"choices\":[{\"delta\":{\"content\":\"Labs\"}}]}\n\n\
                  data: [DONE]\n\n";
    let seen = std::sync::Mutex::new(Vec::new());
    let text = super::read_events(events.as_bytes(), &|chars, so_far| {
        seen.lock().unwrap().push((chars, so_far.to_string()))
    })
    .unwrap();
    assert_eq!(text, "Привет, Labs");
    assert_eq!(
        *seen.lock().unwrap(),
        [
            (8, "Привет, ".to_string()),
            (12, "Привет, Labs".to_string())
        ]
    );
}
