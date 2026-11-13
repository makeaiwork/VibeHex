//! What a turn may read of the person's recordings, and how it asks them
//! when they haven't said: only `assistant.read_recordings` opens them,
//! and a turn that finds it unset asks once, mid-answer.

use crate::assistant::recordings::{Allowed, Sources, Step};
use crate::dictation::vocabulary::VocabularyEntry;

/// What a turn may read of the person's recordings.
#[derive(Clone, Debug, Default)]
pub enum Access {
    /// They allowed it (`assistant.read_recordings`).
    Open {
        allowed: Allowed,
        sources: Sources,
        /// Their Vocabulary, so a term is found however it was written;
        /// the search reads it, the model never does.
        vocabulary: Vec<VocabularyEntry>,
    },
    /// They haven't said: the first call that needs them asks, once.
    Ask {
        sources: Sources,
        vocabulary: Vec<VocabularyEntry>,
    },
    /// They said no on the card in this conversation, or there is nowhere
    /// to read from.
    #[default]
    Closed,
}

impl Access {
    /// What `settings` allow at `sources`: open once the person said yes,
    /// else to be asked.
    pub fn of(settings: &crate::app_settings::AppSettings, sources: Option<Sources>) -> Self {
        let Some(sources) = sources else {
            return Self::Closed;
        };
        let vocabulary = settings.vocabulary.clone();
        match Allowed::from_settings(&settings.assistant) {
            Some(allowed) => Self::Open {
                allowed,
                sources,
                vocabulary,
            },
            None => Self::Ask {
                sources,
                vocabulary,
            },
        }
    }
}

/// What the person said when asked mid-answer.
pub enum Consent {
    Allowed(Allowed),
    Declined,
    /// They stopped the answer instead.
    Stopped,
    /// They didn't answer within `live::CONSENT_WAIT`: the turn answers
    /// without the recordings, and nothing is decided, so a later
    /// question may ask again.
    Unanswered,
}

/// What a tool reaches while it runs: the person, for their leave, and
/// the pane, for a search's steps as they land.
pub trait Live {
    /// Asks whether the recordings may be read, and waits for the answer.
    fn consent(&self) -> Consent;
    fn step(&self, step: &Step);
}

/// What the model reads when the recordings may not be read.
pub const CLOSED: &str = "Not read: the person hasn't allowed you to read their meetings, \
files, and dictations. Call no recordings tool again in this conversation. Say in one sentence \
that you can't read them and that **Assistant › Settings › Your meetings, files, and \
dictations** allows it; then answer what the guide covers, if anything.";

/// What the model reads when the person was asked and didn't answer.
pub const UNANSWERED: &str = "Not read: the person was asked whether you may read their \
meetings, files, and dictations, and gave no answer. Call no recordings tool again in this \
answer. Say in one sentence that you answer without their recordings because access to them \
wasn't given, and that they can allow it when asked again or in **Assistant › Settings › Your \
meetings, files, and dictations**; then answer what the guide covers, if anything.";
