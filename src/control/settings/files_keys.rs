//! `hex settings` keys for Labs › Files.

use super::{Key, Kind, key, optional};

pub(super) const FILES_KEYS: &[Key] = &[
    key(
        "labs.files",
        Kind::Bool,
        "Labs: the Files pane transcribes dropped audio and video files, cleans them up, and reworks the text with your requests (macOS)",
    ),
    optional(
        "files.transcription",
        Kind::Json,
        "Labs › Files: the model and language files are transcribed with, as {\"model\": …, \"language\": …}; none follows dictation's model",
    ),
    optional(
        "files.language",
        Kind::Text,
        "Labs › Files: the language files are transcribed in, for models that take a language hint; none follows the model's selection",
    ),
    key(
        "files.use_vocabulary",
        Kind::Bool,
        "Labs › Files: transcription and cleanup read the shared Vocabulary",
    ),
    key(
        "files.show_timestamps",
        Kind::Bool,
        "Labs › Files: a file's Original tab opens with its timestamps",
    ),
    key(
        "files.cleanup_after",
        Kind::Bool,
        "Labs › Files: a file is cleaned up as soon as it is transcribed",
    ),
    optional(
        "files.cleanup_engine",
        Kind::Text,
        "Labs › Files: where cleanup runs: local:<model id>, openrouter, custom, codex (dictation's Codex model) or codex:<model id>, or opencode; none follows dictation's AI cleanup",
    ),
    optional(
        "files.request_engine",
        Kind::Text,
        "Labs › Files: where requests (Summary, Action items, your own) run, in files.cleanup_engine's form; none follows cleanup's",
    ),
    key(
        "files.request_effort",
        Kind::Text,
        "Labs › Files: how hard a Codex model thinks for requests: low, medium, or high; empty for medium",
    ),
    key(
        "files.use_default_prompt",
        Kind::Bool,
        "Labs › Files: cleanup follows the Global mode's instructions; off, it follows files.prompt",
    ),
    key(
        "files.prompt",
        Kind::Text,
        "Labs › Files: cleanup's own instructions, while files.use_default_prompt is off",
    ),
    key(
        "files.quick_prompts",
        Kind::Json,
        "Labs › Files: the requests above a file's text, as [{\"label\": …, \"prompt\": …}]",
    ),
    optional(
        "files.last_folder",
        Kind::Text,
        "Labs › Files: the folder Choose file… opens in",
    ),
];
