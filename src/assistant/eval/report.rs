//! What a run of the scenarios made, as JSON and as a table to read.

use serde_json::{Value, json};

use super::harness::{Ran, StepRan, Verdict};
use crate::assistant::changes::{Card, CardKind, vocab};

fn verdicts(checks: &[(String, Verdict)]) -> Vec<Value> {
    checks
        .iter()
        .map(|(name, verdict)| match verdict {
            Ok(()) => json!({"check": name, "pass": true}),
            Err(why) => json!({"check": name, "pass": false, "why": why}),
        })
        .collect()
}

/// A card in a few words: `labs.files=true`, `transcription.model
/// (choice)`, `vocabulary: Codex, GitHub`.
fn cards(cards: &[Card]) -> Vec<String> {
    cards
        .iter()
        .flat_map(|card| {
            card.items.iter().map(move |item| match &card.kind {
                CardKind::Vocabulary | CardKind::VocabularyEdit => {
                    let terms: Vec<String> = vocab::terms(&item.to)
                        .into_iter()
                        .map(|term| term.term)
                        .collect();
                    format!("vocabulary: {}", terms.join(", "))
                }
                CardKind::Choice { .. } => format!("{} (choice)", item.key),
                CardKind::Prompt => format!("{} (instructions)", item.key),
                CardKind::Plan if item.key == vocab::KEY => {
                    let terms: Vec<String> = vocab::terms(&item.to)
                        .into_iter()
                        .map(|term| term.term)
                        .collect();
                    format!(
                        "{} term {}",
                        if item.included { "[x]" } else { "[ ]" },
                        terms.join(", ")
                    )
                }
                CardKind::Plan => format!(
                    "{} {}={}",
                    if item.included { "[x]" } else { "[ ]" },
                    item.key,
                    item.to
                ),
                CardKind::Single => format!("{}={}", item.key, item.to),
            })
        })
        .collect()
}

fn step(step: &StepRan) -> Value {
    json!({
        "said": step.said,
        "route": step.route,
        "asked_a_model": step.asked,
        "checks": verdicts(&step.checks),
        "cards": cards(&step.turn.cards),
        "asks": step.turn.asks.iter().map(|ask| ask.id.clone()).collect::<Vec<_>>(),
        "dropped": crate::assistant::offer::recorded(&step.turn.knew),
        "refused": step.turn.refused.iter().map(|refused| format!("{} ({})", refused.line, refused.reason())).collect::<Vec<_>>(),
        "steps": step.turn.steps.len(),
        "sources": step.turn.sources,
        "web_sources": step.turn.web.sources,
        "agent": !step.turn.trace.is_empty(),
        "requests": step.requests,
        "called": step.called,
        "trace": step.turn.trace,
        "first_event_s": step.first_event,
        "first_read_s": step.first_read,
        "first_word_s": step.first_word,
        "total_s": step.seconds,
        "prompt_chars": step.prompt_chars,
        "error": step.turn.error,
        "answer": step.turn.answer,
    })
}

/// The whole run as JSON.
pub fn json(layer: &str, model: &str, runs: &[Ran]) -> Value {
    let checks: usize = runs
        .iter()
        .map(|ran| {
            ran.app.len()
                + ran
                    .steps
                    .iter()
                    .map(|step| step.checks.len())
                    .sum::<usize>()
        })
        .sum();
    let failures: Vec<String> = runs.iter().flat_map(Ran::failures).collect();
    json!({
        "layer": layer,
        "model": model,
        "summary": {
            "scenarios": runs.len(),
            "passed": runs.iter().filter(|ran| ran.passed()).count(),
            "checks": checks,
            "failed_checks": failures.len(),
        },
        "failures": failures,
        "scenarios": runs.iter().map(|ran| json!({
            "id": ran.id,
            "about": ran.about,
            "passed": ran.passed(),
            "app": verdicts(&ran.app),
            "steps": ran.steps.iter().map(step).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn passed(checks: &[(String, Verdict)]) -> String {
    let ok = checks.iter().filter(|(_, verdict)| verdict.is_ok()).count();
    format!("{ok}/{}", checks.len())
}

fn seconds(seconds: Option<f32>) -> String {
    seconds.map_or_else(|| "-".into(), |seconds| format!("{seconds:.1} s"))
}

/// The run as a table, a row a step, then each failure with why. `reqs`
/// is the requests a turn made; `read`, when the first thing its model
/// asked to read showed as a step; `word`, when the answer's first words
/// did.
pub fn table(runs: &[Ran]) -> String {
    let mut out = format!(
        "{:<28} {:<5} {:<20} {:>6} {:>4}  {:>7} {:>7} {:>7} {:>8} {:>8}  {}\n",
        "scenario",
        "step",
        "route",
        "checks",
        "reqs",
        "first",
        "read",
        "word",
        "total",
        "prompt",
        "cards, then the tools called"
    );
    for ran in runs {
        if !ran.app.is_empty() {
            out.push_str(&format!(
                "{:<28} {:<5} {:<20} {:>6}\n",
                ran.id,
                "app",
                "no model",
                passed(&ran.app)
            ));
        }
        for (index, step) in ran.steps.iter().enumerate() {
            let mut cards = cards(&step.turn.cards);
            if !step.called.is_empty() {
                cards.push(format!("called {}", step.called.join(", ")));
            }
            out.push_str(&format!(
                "{:<28} {:<5} {:<20} {:>6} {:>4}  {:>7} {:>7} {:>7} {:>8} {:>8}  {}\n",
                ran.id,
                index + 1,
                step.route,
                passed(&step.checks),
                match step.requests {
                    0 => "-".into(),
                    requests => requests.to_string(),
                },
                seconds(step.first_event),
                seconds(step.first_read),
                seconds(step.first_word),
                seconds(step.asked.then_some(step.seconds)),
                match step.prompt_chars {
                    0 => "-".into(),
                    chars => format!("{:.1} k", chars as f32 / 1000.0),
                },
                match cards.is_empty() {
                    true => "-".into(),
                    false => cards.join("; "),
                },
            ));
        }
    }
    // What the gate dropped, a line a proposal.
    for ran in runs {
        for (index, step) in ran.steps.iter().enumerate() {
            for line in crate::assistant::offer::recorded(&step.turn.knew) {
                out.push_str(&format!("dropped {} step {}: {line}\n", ran.id, index + 1));
            }
        }
    }
    let failures: Vec<String> = runs.iter().flat_map(Ran::failures).collect();
    let scenarios = runs.iter().filter(|ran| ran.passed()).count();
    out.push_str(&format!(
        "\n{scenarios} of {} scenarios pass, {} failed checks\n",
        runs.len(),
        failures.len()
    ));
    for failure in &failures {
        out.push_str(&format!("FAIL {failure}\n"));
    }
    out
}
