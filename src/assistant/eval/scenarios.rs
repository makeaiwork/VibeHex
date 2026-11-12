//! Every scenario, and the two ways they run: offline in `scripts/check`
//! (`the_scenarios_hold_offline`), and live on a real engine with
//! `scripts/assistant-eval` (`evaluate_assistant_scenarios`).
//!
//! - `HEX_ASSISTANT_EVAL_ENGINE`: `codex`, `codex:<model>`, `local:<id>`,
//!   `openrouter`, `custom`, or `opencode`.
//! - `HEX_ASSISTANT_EVAL_EFFORT`: Codex's reasoning, `low` by default.
//! - `HEX_ASSISTANT_EVAL_ONLY`: scenario ids to run, comma separated, each
//!   matching from its start (`08,11-returning`).
//! - `HEX_ASSISTANT_EVAL_OUT`: where the JSON report goes.
//! - `HEX_ASSISTANT_EVAL_DUMP`: a folder for what each step sent the model.

use std::path::PathBuf;

use super::harness::{Mode, Ran, Scenario, run};
use super::{answers, gate, report, setups};
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

/// The scenarios `HEX_ASSISTANT_EVAL_ONLY` names, or all of them.
fn chosen() -> Vec<Scenario> {
    let only: Vec<String> = std::env::var("HEX_ASSISTANT_EVAL_ONLY")
        .unwrap_or_default()
        .split(',')
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
        .collect();
    all()
        .into_iter()
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
    let scenarios = chosen();
    let runs: Vec<Ran> = scenarios
        .iter()
        .map(|scenario| run(scenario, &Mode::Canned))
        .collect();
    let failures: Vec<String> = runs.iter().flat_map(Ran::failures).collect();
    if !failures.is_empty() || std::env::var_os("HEX_ASSISTANT_EVAL_OUT").is_some() {
        eprintln!("{}", report::table(&runs));
        eprintln!("report: {}", write("offline", "played", &runs).display());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Scenario ids are unique, and each holds something.
#[test]
fn every_scenario_checks_something() {
    let all = all();
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
    assert!(all.len() >= 19, "{} scenarios", all.len());
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
    for scenario in chosen() {
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
