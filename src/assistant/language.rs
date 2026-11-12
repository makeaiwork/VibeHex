//! The language Assistant speaks before the person writes: the speech
//! recognition language when it is a specific one (the one set, or the
//! only one the speech model knows), else the window's. Its own words
//! (the start page's questions, setup's questions and chips, the cards'
//! buttons, the steps) are made in it through `i18n::speaking`; a place in
//! the window keeps the window's words. The model is told it too, and
//! answers in the language of the person's latest message when that is
//! clear.

use crate::app_settings::AppSettings;
use crate::desktop::i18n;
use crate::transcription::models::{self, AUTO_LANGUAGE};

/// The speech recognition language when it is a specific one: `ru`, `it`.
pub fn spoken(settings: &AppSettings) -> Option<String> {
    let selection = &settings.transcription;
    let chosen = selection.language.trim();
    if !chosen.is_empty() && chosen != AUTO_LANGUAGE {
        return Some(chosen.to_owned());
    }
    match models::definition(selection.model).languages {
        [only] if *only != "*" => Some((*only).to_owned()),
        _ => None,
    }
}

/// Assistant's own strings in its language for `settings` until the guard
/// drops: a press that writes words into a conversation.
pub fn speak(settings: &AppSettings) -> i18n::Speaking {
    i18n::speak(spoken(settings).as_deref())
}

/// Runs `f` with Assistant's own strings in its language for `settings`.
pub fn within<T>(settings: &AppSettings, f: impl FnOnce() -> T) -> T {
    i18n::speaking(spoken(settings).as_deref(), f)
}

/// The same, for a question that may carry no settings: the window's then.
pub fn within_maybe<T>(settings: Option<&AppSettings>, f: impl FnOnce() -> T) -> T {
    match settings {
        Some(settings) => within(settings, f),
        None => f(),
    }
}

/// Assistant's default language in English, for the model: `Russian`.
pub fn name(settings: Option<&AppSettings>) -> String {
    let code = settings
        .and_then(spoken)
        .unwrap_or_else(|| i18n::window_code().to_owned());
    match code.as_str() {
        "zh" => "Simplified Chinese".to_owned(),
        "zh-Hant" => "Traditional Chinese".to_owned(),
        "pt" => "Portuguese (Brazil)".to_owned(),
        code => crate::transcription::languages::language_name(code).to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::i18n::tr;
    use crate::transcription::models::TranscriptionModelId;

    fn speaking(language: &str) -> AppSettings {
        let mut settings = AppSettings::default();
        settings.transcription.model = TranscriptionModelId::ParakeetV3;
        settings.transcription.language = language.into();
        settings
    }

    #[test]
    fn a_specific_recognition_language_leads_and_auto_follows_the_window() {
        crate::desktop::i18n::apply(Some("en"));
        let russian = speaking("ru");
        assert_eq!(spoken(&russian).as_deref(), Some("ru"));
        assert_eq!(within(&russian, || tr("Not now").to_owned()), "Не сейчас");
        assert_eq!(name(Some(&russian)), "Russian");

        let auto = speaking("auto");
        assert_eq!(spoken(&auto), None);
        assert_eq!(within(&auto, || tr("Not now").to_owned()), "Not now");
        assert_eq!(name(Some(&auto)), "English");
        crate::desktop::i18n::apply(Some("de"));
        assert_eq!(within(&auto, || tr("Not now").to_owned()), "Nicht jetzt");
        assert_eq!(name(Some(&auto)), "German");
        assert_eq!(name(None), "German");
        crate::desktop::i18n::apply(Some("en"));

        // No table for Italian: English words, and the model is told Italian.
        let italian = speaking("it");
        assert_eq!(within(&italian, || tr("Not now").to_owned()), "Not now");
        assert_eq!(name(Some(&italian)), "Italian");

        // A speech model that knows one language speaks it.
        let mut gigaam = speaking("auto");
        gigaam.transcription.model = TranscriptionModelId::GigaAmV3;
        assert_eq!(spoken(&gigaam).as_deref(), Some("ru"));
    }
}
