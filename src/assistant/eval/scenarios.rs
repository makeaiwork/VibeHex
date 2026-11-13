//! Every scenario, and the ways they run: offline in `scripts/check`, and
//! live on a real engine with `scripts/assistant-eval`
//! (`evaluate_assistant_scenarios`).
//!
//! Offline there are three passes. The reported problems run as the
//! router answers them, one request each (`the_scenarios_hold_offline`).
//! The agent's scenarios run through its loop, each a script of replies
//! (`the_agent_scenarios_hold_offline`). And the reported problems run
//! through the loop too, held only to what every step is held to, the
//! privacy rule among them (`every_scenario_keeps_its_privacy_as_an_agent`).
//!
//! Live, both sets run the way the engine answers: as an agent where
//! `assistant.agent` has it so, or `HEX_ASSISTANT_AGENT=1` forces it; one
//! request with `HEX_ASSISTANT_AGENT=0`. The same scenarios run both ways
//! are how the two are compared.
//!
//! - `HEX_ASSISTANT_EVAL_ENGINE`: `codex`, `codex:<model>`, `local:<id>`,
//!   `openrouter`, `custom`, or `opencode`.
//! - `HEX_ASSISTANT_EVAL_EFFORT`: Codex's reasoning, `low` by default.
//! - `HEX_ASSISTANT_EVAL_ONLY`: scenario ids to run, comma separated, each
//!   matching from its start (`08,11-returning`).
//! - `HEX_ASSISTANT_EVAL_OUT`: where the JSON report goes.
//! - `HEX_ASSISTANT_EVAL_DUMP`: a folder for what each step sent the model.

use std::path::PathBuf;

use super::harness::{Held, Mode, Ran, Scenario, run, run_held};
use super::{agent, answers, gate, report, setups};
use crate::app_settings::AppSettings;
use crate::files::rewrite::Engine;

/// Scenarios about what only a Mac has (Add to vocabulary from a selection
/// and its shortcut). The catalog a prompt carries is this OS's, so
/// elsewhere there is nothing for them to hold.
const MAC_ONLY: &[&str] = &["07-words-wrong"];

/// Every scenario this OS can hold, in the order of their ids.
pub fn all() -> Vec<Scenario> {
    let mut all = answers::scenarios();
    all.extend(setups::scenarios());
    all.extend(gate::scenarios());
    all.retain(|scenario| cfg!(target_os = "macos") || !MAC_ONLY.contains(&scenario.id));
    all.sort_by_key(|scenario| scenario.id);
    all
}

/// The agent's own scenarios (`eval::agent`), in the order of their ids.
pub fn agent() -> Vec<Scenario> {
    let mut all = agent::scenarios();
    all.sort_by_key(|scenario| scenario.id);
    all
}

/// Those of `all` that `HEX_ASSISTANT_EVAL_ONLY` names, or all of them.
fn chosen(all: Vec<Scenario>) -> Vec<Scenario> {
    let only: Vec<String> = std::env::var("HEX_ASSISTANT_EVAL_ONLY")
        .unwrap_or_default()
        .split(',')
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
        .collect();
    all.into_iter()
        .filter(|scenario| only.is_empty() || only.iter().any(|id| scenario.id.starts_with(id)))
        .collect()
}

fn write(layer: &str, model: &str, runs: &[Ran]) -> PathBuf {
    let out = std::env::var("HEX_ASSISTANT_EVAL_OUT").map_or_else(
        |_| std::env::temp_dir().join(format!("assistant-scenarios-{layer}.json")),
        PathBuf::from,
    );
    let report = report::json(layer, model, runs);
    std::fs::write(
        &out,
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    )
    .ok();
    out
}

/// Every scenario with the replies a model is played as writing: the
/// routing, what goes into the prompt, and what the app makes of a reply.
#[test]
fn the_scenarios_hold_offline() {
    let scenarios = chosen(all());
    let runs: Vec<Ran> = scenarios
        .iter()
        .map(|scenario| run(scenario, &Mode::Canned))
        .collect();
    held(&runs, "offline");
}

/// Prints the table and writes the report when a check failed (or a
/// report was asked for), then fails on any failure.
fn held(runs: &[Ran], layer: &str) {
    let failures: Vec<String> = runs.iter().flat_map(Ran::failures).collect();
    if !failures.is_empty() || std::env::var_os("HEX_ASSISTANT_EVAL_OUT").is_some() {
        eprintln!("{}", report::table(runs));
        eprintln!("report: {}", write(layer, "played", runs).display());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The agent's scenarios, each a script of replies played through its
/// loop: what a call reads, what the next request carries, the budgets,
/// consent mid-answer, and the web going off once the prompt holds what is
/// the person's own.
#[test]
fn the_agent_scenarios_hold_offline() {
    let runs: Vec<Ran> = chosen(agent())
        .iter()
        .map(|scenario| run(scenario, &Mode::Agent))
        .collect();
    held(&runs, "offline-agent");
}

/// Every reported problem, played through the agent's loop: its reply is
/// an answer at once, and each step is held to what every step is held to
/// (no error, no card to the value set, nothing this computer can't use,
/// and never the web once the prompt holds what is the person's own).
/// Their own checks are the router's, on what its one prompt carries.
#[test]
fn every_scenario_keeps_its_privacy_as_an_agent() {
    let runs: Vec<Ran> = chosen(all())
        .iter()
        .map(|scenario| run_held(scenario, &Mode::Agent, Held::Always))
        .collect();
    held(&runs, "offline-as-agent");
}

/// Scenario ids are unique, and each holds something.
#[test]
fn every_scenario_checks_something() {
    let mut all = all();
    let router = all.len();
    all.extend(agent());
    for (index, scenario) in all.iter().enumerate() {
        assert!(
            all[index + 1..].iter().all(|other| other.id != scenario.id),
            "{} twice",
            scenario.id
        );
        let checks: usize = scenario.steps.iter().map(|step| step.checks.len()).sum();
        assert!(
            scenario.app.len() + checks > 0,
            "{} checks nothing",
            scenario.id
        );
        assert!(!scenario.about.is_empty(), "{}", scenario.id);
    }
    assert!(router >= 19, "{router} scenarios");
    assert!(
        all.len() - router >= 10,
        "{} agent scenarios",
        all.len() - router
    );
}

#[test]
#[ignore = "asks a real engine; run scripts/assistant-eval"]
fn evaluate_assistant_scenarios() {
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
    let model = engine.model_name();
    let mut runs = Vec::new();
    let mut scenarios = all();
    scenarios.extend(agent());
    for scenario in chosen(scenarios) {
        let ran = run(&scenario, &Mode::Live(Box::new(engine.clone())));
        eprintln!(
            "{:<28} {}",
            ran.id,
            if ran.passed() { "pass" } else { "FAIL" }
        );
        runs.push(ran);
    }
    eprintln!("\n{} at {effort} effort\n{}", model, report::table(&runs));
    eprintln!("report: {}", write("live", &model, &runs).display());
    let failures: Vec<String> = runs.iter().flat_map(Ran::failures).collect();
    assert!(failures.is_empty(), "{} failed checks", failures.len());
}
