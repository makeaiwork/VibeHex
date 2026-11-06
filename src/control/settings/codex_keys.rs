//! `hex settings` keys for AI cleanup on Codex.

use super::{Key, Kind, key};

pub(super) const CODEX_KEYS: &[Key] = &[
    key(
        "dictation_processing.server.codex_model",
        Kind::Text,
        "The Codex model cleanup uses, such as gpt-6-luna; empty for Codex's default",
    ),
    key(
        "dictation_processing.server.codex_effort",
        Kind::Text,
        "How hard the Codex model thinks first: low, medium, or high; empty for the model's default",
    ),
    key(
        "dictation_processing.server.codex_wait_seconds",
        Kind::Integer { min: 0, max: 600 },
        "How many seconds a dictation waits for Codex before it pastes as heard, instead of the mode's deadline; 0 for 60",
    ),
];
