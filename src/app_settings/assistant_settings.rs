//! Assistant's own settings: whether its row shows at the bottom of the
//! sidebar, which model answers, how hard it thinks, and whether
//! conversations are kept. Its model is its own: choosing one here never
//! changes what dictation's AI cleanup runs on.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AssistantSettings {
    /// The Assistant row in the sidebar. On by default and not in Labs.
    pub enabled: bool,
    /// Where answers run, in `files.request_engine`'s form (`codex`,
    /// `codex:<id>`, `local:<id>`, `openrouter`, `custom`, `opencode`);
    /// `None` asks the engine dictation's AI cleanup uses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// How hard a Codex model thinks: low, medium, or high; empty for
    /// medium.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
    /// Conversations are saved on this computer; off, they last until
    /// VibeHex quits.
    pub keep_conversations: bool,
    /// Questions about the person's meetings, Files texts, and dictations
    /// may search them on this computer and send the passages found to the
    /// model asked. `None` until the person is first asked; only `Some(true)`
    /// lets anything be read (`assistant::recordings::Allowed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_recordings: Option<bool>,
    /// The model reads the person's current switches, choices, numbers,
    /// and speech models with each question, so it suggests only what
    /// isn't on yet; never a path, a key, or a list.
    pub send_settings: bool,
}

impl Default for AssistantSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: None,
            effort: String::new(),
            keep_conversations: true,
            read_recordings: None,
            send_settings: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_is_on_and_keeps_conversations_by_default() {
        let settings: AssistantSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, AssistantSettings::default());
        assert!(settings.enabled && settings.keep_conversations && settings.send_settings);
        assert_eq!(
            serde_json::to_string(&settings).unwrap(),
            r#"{"enabled":true,"keep_conversations":true,"send_settings":true}"#
        );
    }
}
