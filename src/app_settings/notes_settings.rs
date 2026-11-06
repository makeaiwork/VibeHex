//! Who writes meeting notes and from what prompt. A writer is one of the
//! engines AI cleanup set up (`codex:<model>`, `openrouter`, `custom`,
//! `local:<id>`, `opencode`), as Files names them; none keeps the older
//! choice of Apple Intelligence or OpenCode. The summary prompt starts from
//! Hex's default in the speech model's language; an edited one is kept per
//! language.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct MeetingNotesSettings {
    /// The engine that writes notes, titles, and live chapters, in Files'
    /// form; `None` for Apple Intelligence or OpenCode as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writer: Option<String>,
    /// How hard a Codex model thinks: `low`, `medium`, or `high`; empty
    /// means low, since notes come after every meeting.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
    /// Summary prompts the person edited, by prompt language.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub prompts: BTreeMap<String, String>,
}

impl MeetingNotesSettings {
    /// The summary prompt for `language`: the edited one, else Hex's.
    pub fn prompt(&self, language: &str) -> String {
        match self.prompts.get(language) {
            Some(prompt) if !prompt.trim().is_empty() => prompt.trim().to_owned(),
            _ => default_prompt(language).to_owned(),
        }
    }

    /// Whether the prompt for `language` was edited.
    pub fn edited(&self, language: &str) -> bool {
        self.prompts
            .get(language)
            .is_some_and(|prompt| prompt.trim() != default_prompt(language).trim())
    }

    /// The reasoning effort notes ask a Codex model for.
    pub fn effort(&self) -> &str {
        match self.effort.trim() {
            "" => "low",
            effort => effort,
        }
    }
}

/// Questions about a finished meeting: whether its Requests tab and field
/// show, and where a new session's questions run.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct MeetingQuestionsSettings {
    pub enabled: bool,
    /// The engine a new session starts on, in `files.request_engine`'s
    /// form; `None` follows Files › Settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// How hard a Codex model thinks; empty follows Files › Settings.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
}

impl Default for MeetingQuestionsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: None,
            effort: String::new(),
        }
    }
}

fn table() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("prompts/meeting.json"))
            .expect("prompts/meeting.json is valid")
    })
}

/// Hex's summary prompt in `language`, else in English.
pub fn default_prompt(language: &str) -> &'static str {
    let table = table();
    table
        .get(language)
        .or_else(|| table.get("en"))
        .map_or("", String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_settings::prompt_defaults::PROMPT_LANGUAGES;

    #[test]
    fn every_prompt_language_has_a_summary_prompt() {
        for (code, _) in PROMPT_LANGUAGES {
            assert!(table().contains_key(*code), "{code} has no summary prompt");
        }
        assert!(default_prompt("it").starts_with("You write notes"));
    }

    #[test]
    fn an_edited_prompt_is_kept_for_its_language_only() {
        let mut notes = MeetingNotesSettings::default();
        assert!(notes.prompt("ru").starts_with("Ты пишешь заметки"));
        notes.prompts.insert("ru".into(), "Только задачи.".into());
        assert_eq!(notes.prompt("ru"), "Только задачи.");
        assert!(notes.edited("ru") && !notes.edited("en"));
        assert!(notes.prompt("en").starts_with("You write notes"));
        assert_eq!(notes.effort(), "low");
    }
}
