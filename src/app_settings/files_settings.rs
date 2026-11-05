//! Labs › Files: how dropped audio and video files are transcribed and
//! cleaned up, and the requests offered beside their text.

use serde::{Deserialize, Serialize};

use crate::transcription::models::TranscriptionSelection;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct FilesSettings {
    /// The model files are transcribed with; `None` follows dictation's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription: Option<TranscriptionSelection>,
    /// Transcription and cleanup read the shared Vocabulary.
    pub use_vocabulary: bool,
    /// The text open on a file's Original tab keeps its timestamps.
    pub show_timestamps: bool,
    /// A file is cleaned up as soon as it is transcribed.
    pub cleanup_after: bool,
    /// Where cleanup and requests run: `local:<model id>`, `openrouter`,
    /// `custom`, or `opencode`; `None` follows dictation's AI cleanup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_engine: Option<String>,
    /// Cleanup follows the Global mode's instructions; off, it follows
    /// `prompt`.
    pub use_default_prompt: bool,
    /// Files' own cleanup instructions, while `use_default_prompt` is off.
    pub prompt: String,
    /// The chips above a file's request field.
    pub quick_prompts: Vec<QuickPrompt>,
    /// Where Choose file… opens: the folder of the file chosen last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_folder: Option<String>,
}

/// A request one click away: its chip's label and what it asks.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct QuickPrompt {
    pub label: String,
    pub prompt: String,
}

/// Cleanup's instructions while Files doesn't follow the Global mode's.
pub const DEFAULT_FILES_PROMPT: &str = "Clean up this transcript of a recording: fix punctuation, \
capitalization, and misheard words, remove filler words and false starts, and split it into \
paragraphs. Keep the speaker's words and language; do not summarize or add anything.";

impl Default for FilesSettings {
    fn default() -> Self {
        Self {
            transcription: None,
            use_vocabulary: true,
            show_timestamps: true,
            cleanup_after: true,
            cleanup_engine: None,
            use_default_prompt: true,
            prompt: DEFAULT_FILES_PROMPT.into(),
            quick_prompts: default_quick_prompts(),
            last_folder: None,
        }
    }
}

/// Structure, Summary, and Action items.
pub fn default_quick_prompts() -> Vec<QuickPrompt> {
    [
        (
            "Structure",
            "Structure this text: add headings, group related points, and make lists where \
             it lists things. Keep every fact and the original language.",
        ),
        (
            "Summary",
            "Write a summary of this text in Markdown: the main points as short bullet points \
             under a few headings, in the text's language.",
        ),
        (
            "Action items",
            "List the action items in this text as a Markdown checklist: who does what and by \
             when, where it says. In the text's language.",
        ),
    ]
    .into_iter()
    .map(|(label, prompt)| QuickPrompt {
        label: label.into(),
        prompt: prompt.into(),
    })
    .collect()
}
