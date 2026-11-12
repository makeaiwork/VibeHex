//! Where a typed message goes, from its words alone, and what its answer
//! may read: the one routing the pane and the eval's scenarios share.
//!
//! In order: "set me up like Denis" is the preset; a setup request opens
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
/// oldest first.
pub fn route(question: &str, earlier: &[&str], now_ms: u64) -> Route {
    if preset::preset(question) {
        return Route::Preset;
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
