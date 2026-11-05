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
        "Labs › Files: where cleanup and requests run: local:<model id>, openrouter, custom, or opencode; none follows dictation's AI cleanup",
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
