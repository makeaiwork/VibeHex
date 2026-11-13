//! Round one's language question for whoever is asked: a person whose
//! language is neither Russian nor English sees it first, by name, then
//! English and "Other or mixed" (owner decision 2026-11-13). Their
//! language is the recognition language when it is a specific one
//! (`assistant::language::spoken`), else the system's, else the window's.
//! The Russian options show beside it only when Russian is one of the
//! three; with Russian or English, or a language VibeHex doesn't name,
//! the question is the usual one.

use crate::app_settings::AppSettings;
use crate::assistant::facts::State;
use crate::desktop::i18n;
use crate::transcription::models::{AUTO_LANGUAGE, LANGUAGES};

/// The start of a language option's id: `lang:es`.
pub const PREFIX: &str = "lang:";

/// Whose language round one asks about.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Spoken {
    /// The person's own language when it is neither Russian nor English:
    /// a code of [`LANGUAGES`] (`es`).
    pub own: Option<&'static str>,
    /// Russian is the recognition, system, or window language.
    pub russian: bool,
}

impl Spoken {
    /// For this install: the recognition language, the system's
    /// (`State::system_language`), and the window's.
    pub fn of(settings: &AppSettings, state: &State) -> Self {
        Self::from(
            crate::assistant::language::spoken(settings).as_deref(),
            &state.system_language,
            i18n::window_code(),
        )
    }

    /// From the recognition language (`None` on Auto), the system's, and
    /// the window's. The first one set decides; the Russian options follow
    /// any of the three.
    pub fn from(recognition: Option<&str>, system: &str, window: &str) -> Self {
        let sources = [recognition.unwrap_or_default(), system, window];
        let theirs = sources
            .iter()
            .map(|code| code.trim())
            .find(|code| !code.is_empty())
            .and_then(known);
        Self {
            own: theirs.filter(|code| !matches!(*code, "ru" | "en")),
            russian: sources.iter().any(|code| known(code) == Some("ru")),
        }
    }
}

/// `code` as a language VibeHex names (`es-ES`, `nb`, `zh-Hant` read as
/// `es`, `no`, `zh`), never Auto.
pub fn known(code: &str) -> Option<&'static str> {
    let primary = code
        .split(['-', '_', '.'])
        .next()?
        .trim()
        .to_ascii_lowercase();
    let primary = match primary.as_str() {
        "nb" | "nn" => "no",
        "tl" => "fil",
        "iw" => "he",
        "in" => "id",
        other => other,
    };
    LANGUAGES
        .iter()
        .map(|(known, _)| *known)
        .find(|known| *known == primary && *known != AUTO_LANGUAGE)
}

/// A language's option id: `lang:es`.
pub fn id(code: &str) -> String {
    format!("{PREFIX}{code}")
}

/// The language an option id names: `es` for `lang:es`.
pub fn code(id: &str) -> Option<&'static str> {
    known(id.strip_prefix(PREFIX)?)
}

/// A language's name in English, for `tr`: `Spanish`.
pub fn name(code: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find_map(|(known, name)| (*known == code).then_some(*name))
        .unwrap_or_default()
}

/// A language's name, translated, as it reads inside a sentence: the
/// languages that write their names in lower case (Polish, Spanish,
/// French, Portuguese, Russian) start it so; the chip keeps a capital.
pub fn in_sentence(code: &str) -> String {
    let name = i18n::tr(name(code));
    if !matches!(i18n::speaking_code(), "pl" | "es" | "fr" | "pt" | "ru") {
        return name.to_owned();
    }
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

/// The options of the language question for `spoken`: English labels for
/// the pane to translate, ids fixed.
pub fn options(spoken: Spoken) -> Vec<(String, &'static str)> {
    let Some(own) = spoken.own else {
        return USUAL
            .iter()
            .map(|(id, label)| ((*id).to_owned(), *label))
            .collect();
    };
    let mut options = vec![(id(own), name(own)), ("en".to_owned(), "English")];
    if spoken.russian {
        options.push(("ru".to_owned(), "Russian"));
        options.push(("ru_tech".to_owned(), "Russian with English tech terms"));
    }
    options.push(("mixed".to_owned(), "Other or mixed"));
    options
}

/// The question's options for Russian and English, and for a language
/// VibeHex doesn't name.
pub const USUAL: [(&str, &str); 4] = [
    ("ru", "Russian"),
    ("en", "English"),
    ("ru_tech", "Russian with English tech terms"),
    ("mixed", "Other or mixed"),
];

/// Every language name an option may show, for the translation tables'
/// test.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    LANGUAGES
        .iter()
        .filter(|(code, _)| *code != AUTO_LANGUAGE)
        .map(|(_, name)| *name)
        .collect()
}

#[cfg(test)]
mod tests;
