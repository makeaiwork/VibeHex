//! Where a typed message goes, from its words alone, and what its answer
//! may read: the one routing the pane and the eval's scenarios share.
//!
//! In order: "set me up like Denis" is the preset, where it is offered
//! (Russian recognition only; elsewhere the message is a question like
//! any other); a setup request opens
//! guided setup; a question about VibeHex's releases reads their notes; a
//! question about the person's recordings (its own words, or carried over
//! from the question before) searches them once they allowed it; anything
//! else is a guide question.

use super::{Intent, carried, preset, releases, setup};
use crate::app_settings::AppSettings;
use crate::assistant::recordings::{Allowed, Sources};
use crate::assistant::releases::Asked;
use crate::assistant::worker::{Recall, Search};

/// Where a message goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    /// "Set me up like Denis": the owner's preset as a plan, no model.
    /// Only with Russian recognition (`setup::preset::offered`).
    Preset,
    /// Guided setup, from round one.
    Setup,
    /// What changed in VibeHex: its release notes, never the recordings.
    Releases(Asked),
    /// About the person's recordings, or maybe so.
    Recordings(Intent),
    Guide,
}

/// Where `question` goes after the conversation's `earlier` questions,
/// oldest first, for someone with `settings`.
pub fn route(question: &str, earlier: &[&str], settings: &AppSettings, now_ms: u64) -> Route {
    if preset::preset(question) {
        // Not offered here: an ordinary question, which the guide answers
        // (it says the preset is for Russian dictation), never setup.
        return match preset_not_offered(question, settings) {
            true => Route::Guide,
            false => Route::Preset,
        };
    }
    if setup(question) {
        return Route::Setup;
    }
    if let Some(asked) = releases(question) {
        return Route::Releases(asked);
    }
    match carried(question, earlier, now_ms) {
        Some(intent) => Route::Recordings(intent),
        None => Route::Guide,
    }
}

/// Whether `question` asks for the preset where it isn't offered: its
/// answer is words alone, whatever lines a model adds.
pub fn preset_not_offered(question: &str, settings: &AppSettings) -> bool {
    preset::preset(question) && !crate::assistant::setup::preset::offered(settings)
}

/// What a question about recordings may read.
#[derive(Clone, Debug)]
pub enum Reads {
    Recall(Recall),
    /// Surely about recordings, and they haven't been allowed yet: the
    /// consent card asks first.
    Consent,
}

/// What a question with `intent` reads: the recordings at `sources` once
/// the person allowed it; before that, the card when it is surely about
/// them, else the guide alone.
pub fn reads(
    intent: Intent,
    settings: &AppSettings,
    sources: Option<Sources>,
    now_ms: u64,
) -> Reads {
    match Allowed::from_settings(&settings.assistant) {
        Some(allowed) => Reads::Recall(match sources {
            Some(sources) => Recall::Search(Box::new(Search {
                allowed,
                sources,
                intent,
                vocabulary: settings.vocabulary.clone(),
                now_ms,
            })),
            None => Recall::Guide,
        }),
        None if intent.sure => Reads::Consent,
        None => Reads::Recall(Recall::Guide),
    }
}
