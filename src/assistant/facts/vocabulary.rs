//! Who reads the Vocabulary, and how much of it: the speech model takes
//! only its first terms while it transcribes, AI cleanup reads the whole
//! list afterwards. Counted by the functions the Vocabulary sheet and
//! transcription use, so Assistant and the sheet say the same number.

use crate::app_settings::AppSettings;
use crate::assistant::changes::parts;
use crate::transcription::models::{self, VocabularyUse};
use serde_json::Value;

/// How the speech model in use takes the list while it transcribes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reads {
    /// A Parakeet-family model is biased toward the first `read` terms,
    /// and only while Labs › Vocabulary biasing is `on`.
    Bias { read: usize, on: bool },
    /// Whisper and Qwen read the first `read` terms as a hint.
    Prompt { read: usize },
    /// The model doesn't use the Vocabulary while transcribing.
    None,
}

impl Reads {
    /// How many terms the model takes, as things are set.
    pub fn read(self) -> usize {
        match self {
            Self::Bias { read, on: true } | Self::Prompt { read } => read,
            Self::Bias { on: false, .. } | Self::None => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vocabulary {
    /// How many terms the list holds.
    pub terms: usize,
    pub speech: Reads,
    /// Clean up with AI is on in the Global mode: cleanup reads every
    /// term with its sounds-like spellings (`processor::vocabulary_lines`
    /// has no cap).
    pub cleanup: bool,
}

impl Vocabulary {
    pub fn of(settings: &AppSettings) -> Self {
        let model = models::definition(settings.transcription.model);
        // What the sheet's "This model reads the first N of M terms" counts.
        let read = model.vocabulary_terms_read(&settings.vocabulary);
        Self {
            terms: settings.vocabulary.len(),
            speech: match model.vocabulary_use() {
                VocabularyUse::Bias => Reads::Bias {
                    read,
                    on: settings.labs.vocabulary_biasing,
                },
                VocabularyUse::Prompt => Reads::Prompt { read },
                VocabularyUse::None => Reads::None,
            },
            cleanup: parts::Part::Cleanup.value(settings) == Value::Bool(true),
        }
    }

    /// Terms the dictation never uses: past what the speech model takes,
    /// with no cleanup to read the rest.
    pub fn unused(&self) -> usize {
        match self.cleanup {
            true => 0,
            false => self.terms - self.speech.read().min(self.terms),
        }
    }
}
