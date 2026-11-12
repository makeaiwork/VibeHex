//! The made-up installs scenarios play on. Nothing here is anybody's real
//! data: the settings are the sample setup's (`setup::sample`, shaped like
//! the owner's: Orukeet in Russian, cleanup on a built-in model, notes and
//! Files on Codex, on a Mac mini with a Magic Mouse), and the recordings
//! are written for the fixtures:
//!
//! - `tests/fixtures/assistant/scenarios/history.json`: Russian speech
//!   with English terms in it, as a person who dictates code writes, with
//!   what the speech model heard and what cleanup wrote. Three dictations
//!   where "AI" was said and cleanup wrote "AI cleanup" (a Vocabulary
//!   spelling catches it); terms cleanup keeps fixing (кодекс to Codex
//!   four times, гитхаб to GitHub, пул реквест to pull request, докер to
//!   Docker, вайп хекс to VibeHex); ordinary English cleanup writes inside
//!   Russian (test, fix, file, ok), which are no terms; ё put back (все
//!   to всё, more often than any other fix), which is spelling; plain
//!   messages.
//! - `tests/fixtures/assistant/recordings`: the recordings eval's ten
//!   meetings, three Files texts, and fifteen dictations.

use std::path::PathBuf;

use super::harness::Fixture;
use crate::app_settings::AppSettings;
use crate::assistant::facts::{Fact, State};
use crate::assistant::recordings::{Allowed, Sources, usage};
use crate::assistant::setup::{self, Answer, past, since};
use crate::desktop::hardware::{Hardware, Pointer, Screens};
use crate::dictation::vocabulary::VocabularyEntry;

/// Friday 27 November 2026, noon local: after every fixture recording.
pub fn now() -> u64 {
    crate::meeting::query::parse_time("2026-11-27", 0, false).unwrap_or_default() + 12 * 3_600_000
}

/// A fixture folder under `tests/fixtures/assistant`.
pub fn folder(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/assistant")
        .join(name)
}

/// A Mac mini: one outside screen without a notch, a Magic Mouse.
pub fn mini() -> Hardware {
    Hardware {
        screens: Some(Screens {
            built_in: false,
            external: 1,
            notch: false,
        }),
        pointers: Some(vec![Pointer {
            name: "Magic Mouse".into(),
            extra_buttons: false,
        }]),
        model: Some("Mac16,11".into()),
    }
}

/// The sample setup on a Mac mini, its settings and recordings readable,
/// Russian speech with code in its History.
pub fn owner() -> Fixture {
    let mut settings = AppSettings::default();
    setup::sample(&mut settings);
    settings.assistant.send_settings = true;
    settings.assistant.read_recordings = Some(true);
    // The entry behind "AI cleanup instead of AI", and two like a real
    // list holds.
    settings.vocabulary.extend([
        VocabularyEntry {
            term: "AI cleanup".into(),
            sounds_like: vec!["эй ай".into(), "ай клинап".into()],
        },
        VocabularyEntry {
            term: "Ghostty".into(),
            sounds_like: vec!["гости".into(), "гостти".into()],
        },
        VocabularyEntry {
            term: "OpenRouter".into(),
            sounds_like: Vec::new(),
        },
    ]);
    Fixture {
        settings,
        state: State {
            os: "macos",
            installed: Fact::Known(vec!["orukeet".into(), "parakeet_v3".into()]),
            codex_signed_in: Fact::Known(true),
            opencode_running: Fact::Known(false),
            hardware: mini(),
            ..State::default()
        },
        recordings: Some(folder("scenarios")),
        now_ms: now(),
        setups: Vec::new(),
    }
}

/// The owner's install with Vocabulary biasing on: 97 terms, of which
/// Orukeet takes the first 64 while it transcribes, and AI cleanup on,
/// reading them all.
pub fn owner_biased() -> Fixture {
    let mut fixture = owner();
    fixture.settings.labs.vocabulary_biasing = true;
    let cleanup = &mut fixture.settings.dictation_processing.default_mode;
    cleanup.post_processing.enabled = true;
    fixture
}

/// The same with Clean up with AI off: the terms past the first 64 are
/// used nowhere.
pub fn owner_biased_no_cleanup() -> Fixture {
    let mut fixture = owner_biased();
    let cleanup = &mut fixture.settings.dictation_processing.default_mode;
    cleanup.post_processing.enabled = false;
    fixture
}

/// The owner's install reading the recordings eval's meetings, Files
/// texts, and dictations.
pub fn owner_with_meetings() -> Fixture {
    Fixture {
        recordings: Some(folder("recordings")),
        ..owner()
    }
}

/// A fresh install on the same Mac mini: every setting as it ships, read
/// by Assistant, nothing recorded yet. Russian speech.
pub fn newcomer() -> Fixture {
    let mut settings = AppSettings::default();
    settings.assistant.send_settings = true;
    settings.transcription.language = "ru".into();
    Fixture {
        state: State {
            os: "macos",
            installed: Fact::Known(vec![settings.transcription.model.as_str().into()]),
            codex_signed_in: Fact::Known(true),
            opencode_running: Fact::Known(false),
            hardware: mini(),
            ..State::default()
        },
        settings,
        recordings: None,
        now_ms: now(),
        setups: Vec::new(),
    }
}

/// The same fresh install with speech recognition in `language`: `en`,
/// or `auto`.
pub fn newcomer_speaking(language: &str) -> Fixture {
    let mut fixture = newcomer();
    fixture.settings.transcription.language = language.into();
    fixture
}

/// Round one answered as a person who dictates code in Russian and
/// records meetings would: Codex for cleanup and notes.
pub fn answers() -> Vec<Answer> {
    let answer = |id: &str, chosen: &[&str]| Answer {
        id: id.into(),
        chosen: chosen.iter().map(|&id| id.into()).collect(),
        ..Default::default()
    };
    vec![
        answer("use", &["dictation_code", "meetings"]),
        answer("language", &["ru"]),
        answer("cleanup", &["codex"]),
        answer("notes", &["codex"]),
    ]
}

/// What the fixture's use shows (`usage::read`), as a returning setup
/// reads it; none while its recordings may not be read.
pub fn used(fixture: &Fixture) -> Option<usage::Usage> {
    let allowed = Allowed::from_settings(&fixture.settings.assistant)?;
    let sources = Sources::at(fixture.recordings.as_deref()?);
    let known: Vec<String> = fixture
        .settings
        .vocabulary
        .iter()
        .flat_map(|entry| std::iter::once(&entry.term).chain(&entry.sounds_like))
        .cloned()
        .collect();
    Some(usage::read(&allowed, &sources, &known, fixture.now_ms))
}

/// What a returning setup opens with on the fixture, after `setups`.
pub fn since(fixture: &Fixture, setups: &[past::Setup]) -> since::Since {
    let last = since::last(setups).cloned();
    since::Since {
        releases: since::releases_after(last.as_ref().map_or(0, |last| last.at_ms)),
        last,
        from: None,
        usage: used(fixture),
    }
}
