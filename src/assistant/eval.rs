//! The live eval, opt in and never in CI: asks a real engine the questions
//! in `tests/fixtures/assistant/questions.jsonl`, then guided setup and a
//! vocabulary request, through the same path the pane takes
//! (`worker::ask`: prompt, reply, parse, the repair round), and reports
//! how the replies parsed. Run it with `scripts/assistant-eval`; it does
//! nothing without `HEX_ASSISTANT_EVAL_ENGINE`.
//!
//! - `HEX_ASSISTANT_EVAL_ENGINE`: `codex`, `codex:<model>`, `local:<id>`,
//!   `openrouter`, `custom`, or `opencode`, read with the person's own
//!   settings (keys, servers, Codex's sign-in).
//! - `HEX_ASSISTANT_EVAL_EFFORT`: Codex's reasoning, `low` by default.
//! - `HEX_ASSISTANT_EVAL_LIMIT`: ask only the first N questions.
//! - `HEX_ASSISTANT_EVAL_OUT`: where the JSON report goes.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::changes::{self, CardKind, catalog, vocab};
use super::setup::{self, Answer, Stage};
use super::store::Turn;
use super::worker::{self, Question, Recall};
use crate::app_settings::AppSettings;
use crate::files::rewrite::Engine;

/// How long one answer may take, repair round included.
const PATIENCE: Duration = Duration::from_secs(300);

struct Case {
    id: String,
    question: String,
    setup: Stage,
    /// Keys a card should change, with the value when it matters.
    keys: Vec<(String, Option<Value>)>,
    /// Terms a vocabulary card should hold.
    terms: Vec<String>,
}

fn cases() -> Vec<Case> {
    let text = include_str!("../../tests/fixtures/assistant/questions.jsonl");
    let mut cases: Vec<Case> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .map(|line| Case {
            id: line["id"].as_str().unwrap_or_default().to_owned(),
            question: line["q"].as_str().unwrap_or_default().to_owned(),
            setup: Stage::None,
            keys: line["changes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|change| {
                    let key = change["key"].as_str()?.to_owned();
                    Some((key, change.get("value").cloned()))
                })
                .collect(),
            terms: Vec::new(),
        })
        .collect();
    let answers = vec![
        Answer {
            id: "use".into(),
            chosen: vec!["dictation_code".into(), "meetings".into()],
        },
        Answer {
            id: "language".into(),
            chosen: vec!["ru".into()],
        },
        Answer {
            id: "cloud".into(),
            chosen: vec!["chatgpt".into()],
        },
    ];
    cases.extend([
        Case {
            id: "setup-ask".into(),
            question: "Помоги мне настроить VibeHex под меня".into(),
            setup: Stage::Ask,
            keys: Vec::new(),
            terms: Vec::new(),
        },
        Case {
            id: "setup-plan".into(),
            question: String::new(),
            setup: Stage::Plan {
                line: setup_line(&answers).unwrap_or_default(),
                answers,
            },
            keys: vec![
                ("transcription.model".into(), Some(json!("gigaam_v3"))),
                ("labs.russian_vibe_coding".into(), Some(json!(true))),
                ("meetings.enabled".into(), Some(json!(true))),
            ],
            terms: Vec::new(),
        },
        Case {
            id: "vocabulary".into(),
            question: "Добавь в словарь Kubernetes, kubectl и Grafana, они всё время пишутся криво"
                .into(),
            setup: Stage::None,
            keys: Vec::new(),
            terms: vec!["Kubernetes".into(), "kubectl".into(), "Grafana".into()],
        },
    ]);
    cases
}

/// The answers as the pane sends them, in the app's own words.
fn setup_line(answers: &[Answer]) -> Option<String> {
    let mut asks = setup::questions(std::env::consts::OS);
    for ask in &mut asks {
        ask.chosen = answers
            .iter()
            .find(|answer| answer.id == ask.id)?
            .chosen
            .clone();
    }
    Some(setup::answers_line(&asks, |_, option| option.label.clone()))
}

fn ask(case: &Case, engine: &Engine) -> Option<(Turn, f32)> {
    let settings = AppSettings::default();
    let context = changes::Context {
        settings: settings.clone(),
        send: true,
        state: catalog::State {
            os: std::env::consts::OS,
            ..catalog::State::default()
        },
    };
    let question = Question {
        conversation: 1,
        question: case.question.clone(),
        earlier: Vec::new(),
        recall: Recall::Guide,
        changes: Some(context),
        setup: case.setup.clone(),
        web: false,
    };
    let started = Instant::now();
    if !worker::ask(question, engine.clone()) {
        return None;
    }
    while started.elapsed() < PATIENCE {
        if let Some((_, turn)) = worker::take_finished().pop() {
            return Some((turn, started.elapsed().as_secs_f32()));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    worker::stop();
    None
}

/// What one reply made of its case.
fn score(case: &Case, turn: &Turn) -> Value {
    let items: Vec<&changes::Item> = turn.cards.iter().flat_map(|card| &card.items).collect();
    let hit = |key: &str, value: &Option<Value>| {
        items.iter().any(|item| {
            let same_key = item.key == key || item.key.starts_with(&format!("{key}."));
            same_key && value.as_ref().is_none_or(|value| item.to == *value)
        })
    };
    let missed: Vec<&str> = case
        .keys
        .iter()
        .filter(|(key, value)| !hit(key, value))
        .map(|(key, _)| key.as_str())
        .collect();
    let terms: Vec<String> = turn
        .cards
        .iter()
        .filter(|card| card.kind == CardKind::Vocabulary)
        .flat_map(|card| &card.items)
        .flat_map(|item| vocab::terms(&item.to))
        .map(|term| term.term)
        .collect();
    let missed_terms: Vec<&String> = case
        .terms
        .iter()
        .filter(|want| !terms.iter().any(|term| term.eq_ignore_ascii_case(want)))
        .collect();
    let asked = match case.setup {
        // Setup's questions are the model's own, not the app's fallback.
        Stage::Ask => turn.asks.len() == 3 && turn.asks.iter().all(|ask| !ask.own),
        _ => true,
    };
    json!({
        "id": case.id,
        "error": turn.error,
        "cards": items.iter().map(|item| item.key.clone()).collect::<Vec<_>>(),
        "terms": terms,
        "refused": turn.refused.iter().map(|refused| format!("{} ({})", refused.line, refused.reason())).collect::<Vec<_>>(),
        "missed": missed,
        "missed_terms": missed_terms,
        "asked": asked,
        "clean": turn.error.is_none() && turn.refused.is_empty() && asked,
        "answer": turn.answer,
    })
}

#[test]
#[ignore = "asks a real engine; run scripts/assistant-eval"]
fn evaluate_assistant_replies() {
    let Ok(choice) = std::env::var("HEX_ASSISTANT_EVAL_ENGINE") else {
        eprintln!("HEX_ASSISTANT_EVAL_ENGINE isn't set: nothing asked");
        return;
    };
    let settings = AppSettings::load().unwrap_or_default();
    let effort = std::env::var("HEX_ASSISTANT_EVAL_EFFORT").unwrap_or_else(|_| "low".into());
    let processing = &settings.dictation_processing;
    let engine = Engine::resolve(
        Some(&choice),
        &processing.server,
        true,
        &processing.default_mode,
    )
    .with_effort(&effort);
    let limit = std::env::var("HEX_ASSISTANT_EVAL_LIMIT")
        .ok()
        .and_then(|limit| limit.parse().ok())
        .unwrap_or(usize::MAX);
    let mut results = Vec::new();
    // The guide's questions, as many as asked for, then setup and terms.
    let (guide, more): (Vec<Case>, Vec<Case>) = cases()
        .into_iter()
        .partition(|case| matches!(case.setup, Stage::None) && case.terms.is_empty());
    for case in guide.iter().take(limit).chain(&more) {
        let result = match ask(case, &engine) {
            Some((turn, seconds)) => {
                let mut result = score(case, &turn);
                result["seconds"] = json!(seconds);
                result
            }
            None => json!({"id": case.id, "error": "no answer in time", "clean": false}),
        };
        eprintln!(
            "{:<28} {} cards {} missed {} refused {}",
            case.id,
            if result["clean"] == json!(true) {
                "clean"
            } else {
                "     "
            },
            result["cards"],
            result["missed"],
            result["refused"],
        );
        results.push(result);
    }
    let clean = results
        .iter()
        .filter(|result| result["clean"] == json!(true))
        .count();
    let missed = results
        .iter()
        .filter(|result| {
            result["missed"]
                .as_array()
                .is_some_and(|missed| !missed.is_empty())
                || result["missed_terms"]
                    .as_array()
                    .is_some_and(|missed| !missed.is_empty())
        })
        .count();
    eprintln!(
        "\n{} replies on {}: {clean} clean, {missed} missed an expected change",
        results.len(),
        engine.model_name()
    );
    let out = std::env::var("HEX_ASSISTANT_EVAL_OUT").map_or_else(
        |_| std::env::temp_dir().join("assistant-eval.json"),
        PathBuf::from,
    );
    let report = json!({"engine": choice, "model": engine.model_name(), "results": results});
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    eprintln!("report: {}", out.display());
}
