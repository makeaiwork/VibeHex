//! Assistant's quality loop (docs/features/assistant.md › Quality loop).
//!
//! - [`scenarios`]: the problems people reported, each as a scenario: a
//!   fixture install (settings, facts, synthetic recordings), a question
//!   or a setup step, and assertions on what came of it (the route, the
//!   prompt, the parsed turn: cards, asks, plan rows, steps, sources,
//!   language), never on wording. They run offline in `scripts/check`,
//!   with a reply a model is played as writing, and live, on a real
//!   engine, with `scripts/assistant-eval`.
//! - [`questions`]: the guide questions of
//!   `tests/fixtures/assistant/questions.jsonl`, live only.
//! - [`replay`]: the app's own heuristics over this machine's real data,
//!   printed; no model, nothing sent, `scripts/assistant-replay`.
//!
//! Every reported Assistant problem becomes a scenario here in the change
//! that fixes it.

mod answers;
mod checks;
mod fixtures;
mod harness;
mod questions;
mod replay;
mod report;
mod scenarios;
mod setups;
mod stages;

pub use harness::sent;
