//! `hex settings` keys for who writes meeting notes and from what prompt.

use super::{Key, Kind, key};

pub(super) const MEETING_NOTES_KEYS: &[Key] = &[
    key(
        "meetings.notes.writer",
        Kind::Text,
        "Who writes meeting notes, titles, and live chapters: codex:<model>, openrouter, custom, local:<id>, or opencode, as set up in Output › AI cleanup; empty for Apple Intelligence or OpenCode as before",
    ),
    key(
        "meetings.notes.effort",
        Kind::Text,
        "How hard a Codex model thinks over notes: low, medium, or high; empty for low",
    ),
    key(
        "meetings.notes.prompts",
        Kind::Json,
        "Summary prompts edited per language, such as {\"ru\": \"…\"}; a language left out uses Hex's",
    ),
];
