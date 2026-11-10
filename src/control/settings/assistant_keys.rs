//! `hex settings` keys for Assistant, which is on by default and not in
//! Labs.

use super::{Key, Kind, key, optional};

pub(super) const ASSISTANT_KEYS: &[Key] = &[
    key(
        "assistant.enabled",
        Kind::Bool,
        "Assistant: its row at the bottom of the sidebar, a chat that answers questions about VibeHex from its guide",
    ),
    optional(
        "assistant.engine",
        Kind::Text,
        "Assistant: where answers run, in files.request_engine's form; none asks the engine dictation's AI cleanup uses. Choosing one never changes dictation's",
    ),
    key(
        "assistant.effort",
        Kind::Text,
        "Assistant: how hard a Codex model thinks: low, medium, or high; empty for medium",
    ),
    key(
        "assistant.keep_conversations",
        Kind::Bool,
        "Assistant: conversations are saved on this computer; off, they last until VibeHex quits",
    ),
    optional(
        "assistant.read_recordings",
        Kind::Bool,
        "Assistant: may answer questions about your meetings, Files texts, and dictations by searching them on this computer and sending the passages found to the model asked; none until you are first asked",
    ),
];
