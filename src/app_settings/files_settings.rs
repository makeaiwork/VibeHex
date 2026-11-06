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
    /// The language files are transcribed in, for models that take a
    /// language hint; `None` follows the model's selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Transcription and cleanup read the shared Vocabulary.
    pub use_vocabulary: bool,
    /// The text open on a file's Original tab keeps its timestamps.
    pub show_timestamps: bool,
    /// A file is cleaned up as soon as it is transcribed.
    pub cleanup_after: bool,
    /// Where cleanup runs: `local:<model id>`, `openrouter`, `custom`,
    /// `codex`, or `opencode`; `None` follows dictation's AI cleanup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_engine: Option<String>,
    /// Where requests run, in `cleanup_engine`'s form; `None` follows
    /// cleanup's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_engine: Option<String>,
    /// How hard a Codex model thinks for requests: `low`, `medium`, or
    /// `high`; empty means medium.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub request_effort: String,
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
            language: None,
            use_vocabulary: true,
            show_timestamps: true,
            cleanup_after: true,
            cleanup_engine: None,
            request_engine: None,
            request_effort: String::new(),
            use_default_prompt: true,
            prompt: DEFAULT_FILES_PROMPT.into(),
            quick_prompts: default_quick_prompts(),
            last_folder: None,
        }
    }
}

impl FilesSettings {
    /// The model and language a new file is transcribed with: `None` while
    /// both follow dictation, so it runs on dictation's loaded model.
    pub fn selection(&self, dictation: &TranscriptionSelection) -> Option<TranscriptionSelection> {
        match (&self.transcription, &self.language) {
            (None, None) => None,
            (Some(selection), None) => Some(selection.clone()),
            (selection, Some(language)) => {
                let mut selection = selection.clone().unwrap_or_else(|| dictation.clone());
                selection.language = language.clone();
                Some(selection)
            }
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

#[cfg(test)]
mod tests {
    use super::FilesSettings;
    use crate::transcription::models::{TranscriptionModelId, TranscriptionSelection};

    #[test]
    fn a_file_follows_dictation_until_its_model_or_language_is_chosen() {
        let dictation = TranscriptionSelection {
            model: TranscriptionModelId::WhisperLargeV3Turbo,
            language: "en".into(),
        };
        let mut files = FilesSettings::default();
        assert_eq!(files.selection(&dictation), None);

        files.language = Some("ru".into());
        let chosen = files.selection(&dictation).unwrap();
        assert_eq!(chosen.model, TranscriptionModelId::WhisperLargeV3Turbo);
        assert_eq!(chosen.language, "ru");

        files.transcription = Some(TranscriptionSelection {
            model: TranscriptionModelId::ParakeetV3,
            language: "de".into(),
        });
        files.language = None;
        let chosen = files.selection(&dictation).unwrap();
        assert_eq!(chosen.model, TranscriptionModelId::ParakeetV3);
        assert_eq!(chosen.language, "de");
    }
}
