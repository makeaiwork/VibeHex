//! Hex's default prompts in each language it writes them in, and which of
//! those languages a speech model speaks. Quick requests (Structure,
//! Summary, Action items) start from the default for the speech model's
//! language; a prompt the person edits stays as they wrote it.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::transcription::models::{AUTO_LANGUAGE, TranscriptionSelection, definition};

/// The languages Hex has default prompts in, with their own names.
pub const PROMPT_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("ru", "Русский"),
    ("uk", "Українська"),
    ("pl", "Polski"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("pt", "Português (Brasil)"),
    ("fr", "Français"),
    ("ja", "日本語"),
    ("zh", "简体中文"),
    ("zh-Hant", "繁體中文"),
];

/// Why the prompts are in the language they are in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Basis {
    /// The speech model's language, as chosen or as the only one it speaks.
    SpeechModel,
    /// The app's language: the speech model detects the language, or Hex
    /// has no prompts in the one it speaks.
    Interface,
}

/// The language prompts are written in for `selection`, and why.
pub fn prompt_language(selection: &TranscriptionSelection) -> (&'static str, Basis) {
    let model = definition(selection.model);
    let chosen = selection.language.as_str();
    let spoken = if !chosen.is_empty() && chosen != AUTO_LANGUAGE {
        Some(chosen)
    } else if let [only] = model.languages
        && *only != "*"
    {
        Some(*only)
    } else {
        None
    };
    let interface = crate::desktop::i18n::vibehex::interface_code();
    match spoken.and_then(|code| known(code, interface)) {
        Some(code) => (code, Basis::SpeechModel),
        None => (
            known(interface, interface).unwrap_or("en"),
            Basis::Interface,
        ),
    }
}

/// `code` as one of [`PROMPT_LANGUAGES`]: Chinese speech follows the
/// interface's script.
fn known(code: &str, interface: &str) -> Option<&'static str> {
    if code == "zh" && interface == "zh-Hant" {
        return Some("zh-Hant");
    }
    PROMPT_LANGUAGES
        .iter()
        .map(|(known, _)| *known)
        .find(|known| *known == code)
}

/// A prompt language's own name.
pub fn language_name(code: &str) -> &'static str {
    PROMPT_LANGUAGES
        .iter()
        .find(|(known, _)| *known == code)
        .map_or("English", |(_, name)| *name)
}

/// The built-in quick requests: the id saved with one, its English name,
/// and its key in `prompts/quick.json`.
pub const QUICK_REQUESTS: &[(&str, &str)] = &[
    ("structure", "Structure"),
    ("summary", "Summary"),
    ("action_items", "Action items"),
];

/// The quick requests Hex first offers, each following its default.
pub fn default_quick_prompts() -> Vec<QuickPrompt> {
    QUICK_REQUESTS
        .iter()
        .map(|(id, label)| QuickPrompt {
            label: (*label).into(),
            prompt: String::new(),
            builtin: (*id).into(),
        })
        .collect()
}

type Table = HashMap<String, HashMap<String, String>>;

fn quick_table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("prompts/quick.json"))
            .expect("prompts/quick.json is valid")
    })
}

/// Built-in request `id`'s default prompt in `language`, else in English.
pub fn quick_default(id: &str, language: &str) -> Option<&'static str> {
    let (_, key) = QUICK_REQUESTS.iter().find(|(known, _)| *known == id)?;
    let table = quick_table();
    [language, "en"]
        .iter()
        .find_map(|language| table.get(*language)?.get(*key))
        .map(String::as_str)
}

/// Whether `prompt` is a built-in request's default in any language, so a
/// session started from a chip shows no question bubble.
pub fn is_quick_default(prompt: &str) -> bool {
    let prompt = prompt.trim();
    LEGACY.iter().any(|(_, _, text)| *text == prompt)
        || quick_table()
            .values()
            .any(|prompts| prompts.values().any(|text| text.trim() == prompt))
}

/// The defaults before they followed the speech language: a saved request
/// still holding one of these follows the new default.
const LEGACY: &[(&str, &str, &str)] = &[
    (
        "structure",
        "Structure",
        "Structure this text: add headings, group related points, and make lists where it lists \
         things. Keep every fact and the original language.",
    ),
    (
        "summary",
        "Summary",
        "Write a summary of this text in Markdown: the main points as short bullet points under a \
         few headings, in the text's language.",
    ),
    (
        "action_items",
        "Action items",
        "List the action items in this text as a Markdown checklist: who does what and by when, \
         where it says. In the text's language.",
    ),
];

/// A request one click away: its chip's label and what it asks.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct QuickPrompt {
    pub label: String,
    /// What it asks; empty while a built-in request follows its default.
    pub prompt: String,
    /// Which built-in request this is (`structure`, `summary`,
    /// `action_items`); empty for the person's own.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub builtin: String,
}

impl QuickPrompt {
    /// The built-in request this is, also for one saved before requests
    /// had ids that still holds its old default.
    pub fn builtin_id(&self) -> Option<&'static str> {
        if !self.builtin.is_empty() {
            return QUICK_REQUESTS
                .iter()
                .map(|(id, _)| *id)
                .find(|id| *id == self.builtin);
        }
        LEGACY
            .iter()
            .find(|(_, label, text)| *label == self.label && *text == self.prompt.trim())
            .map(|(id, _, _)| *id)
    }

    /// Whether it asks its default, in whatever language that is.
    pub fn follows_default(&self) -> bool {
        self.builtin_id().is_some()
            && (self.prompt.trim().is_empty()
                || LEGACY
                    .iter()
                    .any(|(_, _, text)| *text == self.prompt.trim()))
    }

    /// A built-in request whose prompt the person changed.
    pub fn edited(&self) -> bool {
        self.builtin_id().is_some() && !self.follows_default()
    }

    /// What it asks, for prompts in `language`.
    pub fn resolved(&self, language: &str) -> String {
        if self.follows_default()
            && let Some(text) = self.builtin_id().and_then(|id| quick_default(id, language))
        {
            return text.to_owned();
        }
        self.prompt.trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::models::TranscriptionModelId;

    fn selection(model: TranscriptionModelId, language: &str) -> TranscriptionSelection {
        TranscriptionSelection {
            model,
            language: language.into(),
        }
    }

    #[test]
    fn prompts_follow_the_speech_language_then_the_app_language() {
        assert_eq!(
            prompt_language(&selection(TranscriptionModelId::WhisperLargeV3Turbo, "ru")),
            ("ru", Basis::SpeechModel)
        );
        // Detecting the language: the app's, English in tests.
        assert_eq!(
            prompt_language(&selection(TranscriptionModelId::ParakeetV3, "auto")),
            ("en", Basis::Interface)
        );
        // A language Hex has no prompts in.
        assert_eq!(
            prompt_language(&selection(TranscriptionModelId::WhisperLargeV3Turbo, "it")),
            ("en", Basis::Interface)
        );
    }

    #[test]
    fn every_language_has_every_quick_request() {
        for (code, _) in PROMPT_LANGUAGES {
            for (id, _) in QUICK_REQUESTS {
                let text = quick_table().get(*code).and_then(|prompts| {
                    prompts.get(QUICK_REQUESTS.iter().find(|(i, _)| i == id).unwrap().1)
                });
                assert!(
                    text.is_some_and(|text| !text.trim().is_empty()),
                    "{code} has no {id}"
                );
            }
        }
    }

    #[test]
    fn a_built_in_request_follows_its_default_until_edited() {
        let summary = &default_quick_prompts()[1];
        assert!(summary.follows_default());
        assert!(
            summary
                .resolved("ru")
                .starts_with("Сделай краткое содержание")
        );
        assert!(
            summary
                .resolved("it")
                .starts_with("Summarize this transcript")
        );
        assert!(is_quick_default(&summary.resolved("ru")));

        let old = QuickPrompt {
            label: "Summary".into(),
            prompt: LEGACY[1].2.into(),
            builtin: String::new(),
        };
        assert_eq!(old.builtin_id(), Some("summary"));
        assert!(old.follows_default() && !old.edited());

        let edited = QuickPrompt {
            prompt: "Three bullets, no more.".into(),
            ..summary.clone()
        };
        assert!(edited.edited());
        assert_eq!(edited.resolved("ru"), "Three bullets, no more.");

        let own = QuickPrompt {
            label: "Quotes".into(),
            prompt: "Pull out the best quotes.".into(),
            builtin: String::new(),
        };
        assert!(own.builtin_id().is_none() && !own.edited());
    }
}
