//! What happens to a dictation's text before it pastes: Global's steps and
//! the modes chosen by application or website.

use serde::{Deserialize, Serialize};

use super::TextReplacement;
use crate::dictation::vocabulary::{VocabularyEntry, serialize_saved};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DictationProcessingSettings {
    pub default_mode: DictationMode,
    pub modes: Vec<DictationMode>,
    /// Labs › Clean up on your own server: where every mode's rewrite runs.
    pub server: CleanupServerSettings,
}

/// Where dictation cleanup runs while Labs › Clean up on your own server is
/// on. API keys never live here: they sit in the OS credential store
/// (`dictation::cleanup_server::keys`).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct CleanupServerSettings {
    pub runs_on: CleanupRunsOn,
    /// The OpenAI-compatible base URL for `custom`, such as LM Studio's
    /// `http://127.0.0.1:1234/v1`; empty until chosen.
    pub url: String,
    /// The OpenRouter model id, such as `google/gemini-2.5-flash-lite`.
    pub openrouter_model: String,
    /// The model id the custom server serves, such as `gemma-4-e4b-it-mlx`.
    pub custom_model: String,
    /// The downloaded built-in model runs (`local_models::MODELS`); empty
    /// means the recommended one.
    pub local_model: String,
    /// The Codex model id, such as `gpt-6-luna`; empty means Codex's default.
    pub codex_model: String,
    /// How hard the Codex model thinks first: `low`, `medium`, or `high`;
    /// empty means the model's default.
    pub codex_effort: String,
}

/// The service that rewrites dictations.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupRunsOn {
    /// The running OpenCode service, as without the Labs feature.
    #[default]
    Opencode,
    Openrouter,
    /// Any OpenAI-compatible server: LM Studio, Ollama, Groq, Gemini, OpenAI.
    Custom,
    /// A model Hex downloads, on the bundled llama.cpp engine.
    Local,
    /// The Codex CLI on this computer, on the person's ChatGPT plan.
    Codex,
}

impl Default for DictationProcessingSettings {
    fn default() -> Self {
        Self {
            default_mode: DictationMode {
                name: "Global".into(),
                ..Default::default()
            },
            modes: Vec::new(),
            server: CleanupServerSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DictationMode {
    pub name: String,
    pub applications: Vec<String>,
    pub browser_hosts: Vec<String>,
    pub replacements: Vec<TextReplacement>,
    pub transformations: Vec<String>,
    pub post_processing: DictationPostProcessing,
    /// Names heard and spelled here ahead of the shared vocabulary.
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_saved"
    )]
    pub vocabulary: Vec<VocabularyEntry>,
}

/// Persisted OpenCode rewrite settings for one mode.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DictationPostProcessing {
    pub enabled: bool,
    pub prompt: String,
    /// Optional `provider/model` key from the OpenCode catalog.
    pub model: Option<String>,
    pub variant: Option<String>,
    pub deadline_seconds: u64,
}

impl Default for DictationPostProcessing {
    fn default() -> Self {
        Self {
            enabled: false,
            prompt: "Rewrite the transcript into clear, natural text while preserving the speaker's intended meaning.".into(),
            model: None,
            variant: None,
            deadline_seconds: 30,
        }
    }
}
