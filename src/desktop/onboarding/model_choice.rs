//! The first run's speech model choice: the dictation language, the models
//! worth offering for it, the row selected, and what the person decided.
//! A new install preselects the recommendation for the system language, and
//! a preselection is neither a download nor a decision: nothing here starts
//! one, and only a press ([`ModelChoice::start`]) or "Continue without a
//! model" ([`ModelChoice::go_without`]) counts as the person's choice.
//!
//! That choice is remembered beside the other onboarding markers, so a run
//! quit part-way (macOS's own Quit & Reopen after a grant) goes on with the
//! download the person asked for instead of asking again.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::transcription::models::{
    self, Recommendation, TranscriptionModelId, TranscriptionSelection,
};

/// How many catalog models the choice offers for a language: the three the
/// picker's comparison shows.
pub const OFFERED: usize = 3;

/// A model one language's list also offers after those three, with no badge.
/// Russian's is Orukeet: the recommendation there writes Russian only, and
/// Orukeet, a fine-tune of the 25-language Parakeet v3, is what this fork's
/// owner dictates Russian with English terms on. It is here by that choice,
/// not by a measurement: the catalog never recommends it, and Hex has not
/// measured it.
const ALSO_OFFERED: &[(&str, TranscriptionModelId)] = &[("ru", TranscriptionModelId::Orukeet)];

/// The models worth offering for `language`, the recommended one first: the
/// first catalog models in the picker's order that speak it, each with the
/// picker's badge, then the language's [`ALSO_OFFERED`] model. A language
/// only one model speaks offers that one.
pub fn offers(language: &str) -> Vec<(TranscriptionModelId, Option<Recommendation>)> {
    let mut offers: Vec<(TranscriptionModelId, Option<Recommendation>)> =
        models::picker_models(language)
            .into_iter()
            .filter(|(model, _)| !model.id.is_custom() && model.supports_language(language))
            .take(OFFERED)
            .map(|(model, recommendation)| (model.id, recommendation))
            .collect();
    for (_, also) in ALSO_OFFERED.iter().filter(|(code, _)| *code == language) {
        let model = models::definition(*also);
        let listed = offers.iter().any(|(offered, _)| offered == also);
        if !listed && model.available() && model.supports_language(language) {
            offers.push((*also, None));
        }
    }
    offers
}

/// One row the person can select.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pick {
    /// A catalog model: one offered, one from the full list, or one another
    /// app already downloaded.
    Model(TranscriptionModelId),
    /// A compatible file found on this computer that isn't a catalog model,
    /// by its place in the scan.
    Found(usize),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Decision {
    /// Nothing chosen yet: nothing downloads, and nothing behaves as if a
    /// model were chosen.
    #[default]
    Undecided,
    /// The person pressed Download (or Use) on this row.
    Started(Pick),
    /// The person continues without a model.
    Without,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelChoice {
    language: &'static str,
    offered: Vec<(TranscriptionModelId, Option<Recommendation>)>,
    selected: Option<Pick>,
    decision: Decision,
}

impl ModelChoice {
    /// The choice as it opens: `preselected`'s language, its models, and its
    /// model's row selected when that model speaks the language (a new
    /// install's is the recommendation), else the first offer.
    pub fn new(preselected: &TranscriptionSelection) -> Self {
        let language = models::known_language(&preselected.language).unwrap_or("en");
        let mut choice = Self {
            language,
            offered: Vec::new(),
            selected: None,
            decision: Decision::Undecided,
        };
        choice.set_language(language);
        if !preselected.model.is_custom()
            && models::definition(preselected.model).supports_language(language)
        {
            choice.selected = Some(Pick::Model(preselected.model));
        }
        choice
    }

    /// Changes the dictation language: its models are offered and the first,
    /// the recommended one, is selected. A download already asked for goes
    /// on until the person starts another or cancels it.
    pub fn set_language(&mut self, language: &'static str) {
        self.language = language;
        self.offered = offers(language);
        self.selected = self.offered.first().map(|(model, _)| Pick::Model(*model));
    }

    pub fn select(&mut self, pick: Pick) {
        self.selected = Some(pick);
    }

    pub(super) fn start(&mut self, pick: Pick) {
        self.selected = Some(pick);
        self.decision = Decision::Started(pick);
    }

    pub(super) fn undecide(&mut self) {
        self.decision = Decision::Undecided;
    }

    pub(super) fn go_without(&mut self) {
        self.decision = Decision::Without;
    }

    pub fn language(&self) -> &'static str {
        self.language
    }

    pub fn offered(&self) -> &[(TranscriptionModelId, Option<Recommendation>)] {
        &self.offered
    }

    pub fn selected(&self) -> Option<Pick> {
        self.selected
    }

    pub fn decision(&self) -> Decision {
        self.decision
    }

    /// The row the person started, if any.
    pub fn started(&self) -> Option<Pick> {
        match self.decision {
            Decision::Started(pick) => Some(pick),
            _ => None,
        }
    }

    pub fn without(&self) -> bool {
        self.decision == Decision::Without
    }

    /// `model` with the chosen language when it speaks it, else with the
    /// model's own default, as the picker's choice does.
    pub fn selection_for(&self, model: TranscriptionModelId) -> TranscriptionSelection {
        TranscriptionSelection {
            model,
            language: models::definition(model)
                .language_for(self.language)
                .to_owned(),
        }
    }
}

/// What the person decided, as the next launch reads it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Remembered {
    /// They asked for this model.
    Model(TranscriptionSelection),
    /// They continue without one.
    Without,
}

const MARKER: &str = "onboarding-model";

pub fn remember(choice: &Remembered) {
    let result =
        crate::app_paths::support_dir().and_then(|directory| remember_at(&directory, choice));
    if let Err(error) = result {
        tracing::warn!(%error, "could not record the onboarding model choice");
    }
}

pub fn remembered() -> Option<Remembered> {
    remembered_at(&crate::app_paths::support_dir().ok()?)
}

pub fn forget() {
    if let Ok(directory) = crate::app_paths::support_dir() {
        forget_at(&directory);
    }
}

pub(crate) fn remember_at(directory: &Path, choice: &Remembered) -> color_eyre::Result<()> {
    fs::create_dir_all(directory)?;
    fs::write(directory.join(MARKER), serde_json::to_vec(choice)?)?;
    Ok(())
}

/// The remembered choice, while its model is still one Hex can use.
pub(crate) fn remembered_at(directory: &Path) -> Option<Remembered> {
    let data = fs::read(directory.join(MARKER)).ok()?;
    let choice: Remembered = serde_json::from_slice(&data).ok()?;
    match &choice {
        Remembered::Model(selection) if models::validate_saved(selection).is_err() => None,
        _ => Some(choice),
    }
}

pub(crate) fn forget_at(directory: &Path) {
    let _ = fs::remove_file(directory.join(MARKER));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hex-onboarding-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn the_choice_is_remembered_until_forgotten_or_reset() {
        let directory = temp("model-choice");
        assert_eq!(
            remembered_at(&directory),
            None,
            "a new install decided nothing"
        );

        let russian = TranscriptionSelection {
            model: TranscriptionModelId::GigaAmV3,
            language: "ru".into(),
        };
        remember_at(&directory, &Remembered::Model(russian.clone())).unwrap();
        assert_eq!(
            remembered_at(&directory),
            Some(Remembered::Model(russian.clone()))
        );
        // The newest decision replaces the one before it.
        remember_at(&directory, &Remembered::Without).unwrap();
        assert_eq!(remembered_at(&directory), Some(Remembered::Without));
        forget_at(&directory);
        assert_eq!(remembered_at(&directory), None);

        // Run setup again forgets it with the other markers.
        remember_at(&directory, &Remembered::Model(russian)).unwrap();
        super::super::reset_at(&directory).unwrap();
        assert_eq!(remembered_at(&directory), None);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_marker_that_names_no_usable_model_is_no_choice() {
        let directory = temp("model-choice-invalid");
        fs::create_dir_all(&directory).unwrap();
        for stale in [
            &b"not json"[..],
            br#"{"model":{"model":"no_such_model","language":"en"}}"#,
            br#"{"model":{"model":"gigaam_v3","language":"xx"}}"#,
        ] {
            fs::write(directory.join(MARKER), stale).unwrap();
            assert_eq!(
                remembered_at(&directory),
                None,
                "{}",
                String::from_utf8_lossy(stale)
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn russian_also_offers_orukeet_last_and_unbadged() {
        use TranscriptionModelId::{GigaAmV3, Orukeet, ParakeetV3, WhisperLargeV3Turbo};
        let russian = offers("ru");
        let ids: Vec<_> = russian.iter().map(|(model, _)| *model).collect();
        assert_eq!(ids, [GigaAmV3, WhisperLargeV3Turbo, ParakeetV3, Orukeet]);
        assert_eq!(russian[0].1, Some(Recommendation::Recommended));
        assert_eq!(russian[3].1, None, "Orukeet is never badged");
        // The recommendation is still the row a new choice selects.
        let choice = ModelChoice::new(&models::recommended_selection("ru"));
        assert_eq!(choice.selected(), Some(Pick::Model(GigaAmV3)));
        // No other language's list grows, and none lists a model twice.
        for (language, _) in models::LANGUAGES {
            let offered = offers(language);
            let extra = ALSO_OFFERED
                .iter()
                .filter(|(code, _)| code == language)
                .count();
            assert!(offered.len() <= OFFERED + extra, "{language}: {offered:?}");
            if *language != "ru" {
                assert!(offered.len() <= OFFERED, "{language}: {offered:?}");
            }
            for (index, (model, _)) in offered.iter().enumerate() {
                assert!(
                    offered[..index].iter().all(|(other, _)| other != model),
                    "{language} lists {model:?} twice"
                );
                assert!(models::definition(*model).supports_language(language));
            }
        }
    }

    #[test]
    fn a_new_choice_selects_the_preselection_only_when_it_speaks_the_language() {
        let english = models::recommended_selection("en");
        let choice = ModelChoice::new(&english);
        assert_eq!(choice.selected(), Some(Pick::Model(english.model)));
        assert_eq!(choice.decision(), Decision::Undecided);
        assert_eq!(choice.started(), None);

        // A saved selection whose language no list knows falls back to
        // English and its recommendation.
        let odd = TranscriptionSelection {
            model: TranscriptionModelId::GigaAmV3,
            language: "tlh".into(),
        };
        let choice = ModelChoice::new(&odd);
        assert_eq!(choice.language(), "en");
        assert_eq!(choice.selected(), Some(Pick::Model(english.model)));
    }
}
