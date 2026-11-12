//! The facts against the window: where a fact mirrors a row (the models
//! downloaded, who writes notes, where cleanup runs, a shortcut's keys),
//! it is what the window's own code shows for the same settings. A model
//! told one thing while the window shows another is the fault these pin.

use super::super::ShortcutTarget;
use super::super::Ui;
use super::super::files::settings::engine_label;
use super::super::meeting_notes::writer_chosen;
use super::super::scenes::mac_fixture;
use super::super::settings::binding;
use super::changes::context;
use crate::app_settings::{AppSettings, CleanupRunsOn, MeetingSummaries};
use crate::assistant::facts::{Cleanup, Facts, FilesCleanup, Runs};
use crate::files::rewrite::Engine;
use crate::transcription::models::{self, TranscriptionModelId};

/// The windows the facts are held to: the scenes' Mac, the owner's sample
/// setup, and the cases that went wrong for them.
fn windows() -> Vec<(&'static str, Ui)> {
    let scene = || {
        let mut ui = mac_fixture();
        ui.assistant.fixture = true;
        ui.settings.assistant.send_settings = true;
        ui
    };
    let sample = || {
        let mut ui = scene();
        crate::assistant::setup::sample(&mut ui.settings);
        ui.picker.installed = Some(vec![
            TranscriptionModelId::Orukeet,
            TranscriptionModelId::ParakeetV3,
        ]);
        ui
    };
    let mut windows = vec![("the scenes' Mac", scene()), ("the sample setup", sample())];

    // A Codex writer with Summaries left on this Mac.
    let mut ui = sample();
    ui.settings.meetings.summaries = MeetingSummaries::Mac;
    windows.push(("a Codex writer over Summaries", ui));

    // The same writer saved, with the lab that lets it write off.
    let mut ui = sample();
    ui.settings.labs.ai_cleanup_server = false;
    windows.push(("a writer saved with its lab off", ui));

    let mut ui = sample();
    ui.settings.meetings.notes.writer = Some("opencode".into());
    ui.settings.labs.ai_cleanup_server = false;
    windows.push(("OpenCode writes", ui));

    let mut ui = sample();
    ui.settings.meetings.notes.writer = None;
    ui.settings.files.cleanup_engine = None;
    ui.settings.dictation_processing.server.runs_on = CleanupRunsOn::Codex;
    windows.push(("everything on dictation's Codex", ui));

    let mut ui = sample();
    ui.picker.installed = Some(Vec::new());
    ui.settings.labs.files = false;
    ui.settings.labs.vocabulary_quick_add = false;
    ui.settings.vocabulary_add_hotkey = None;
    windows.push(("nothing downloaded, labs off", ui));

    let mut ui = sample();
    ui.settings.assistant.engine = Some("codex".into());
    ui.settings
        .dictation_processing
        .default_mode
        .post_processing
        .enabled = false;
    windows.push(("Assistant on its own engine, cleanup off", ui));
    windows
}

fn facts(ui: &Ui) -> Facts {
    context(ui).facts()
}

/// What "Same as dictation" names in Files and Assistant's settings.
fn dictation(settings: &AppSettings) -> Engine {
    let processing = &settings.dictation_processing;
    Engine::resolve(
        None,
        &processing.server,
        settings.labs.ai_cleanup_server,
        &processing.default_mode,
    )
}

#[test]
fn the_models_downloaded_are_the_ones_the_picker_shows_downloaded() {
    for (name, ui) in windows() {
        let facts = facts(&ui);
        let language = &ui.settings.transcription.language;
        let shown: Vec<&str> = models::all_models()
            .into_iter()
            .filter(|model| {
                model.available() && ui.picker.installed(model, model.language_for(language))
            })
            .map(|model| model.id.as_str())
            .collect();
        let told: Vec<&str> = facts
            .speech
            .downloaded
            .known()
            .unwrap_or_else(|| panic!("{name}: a window knows its models"))
            .iter()
            .map(|model| model.id.as_str())
            .collect();
        assert_eq!(told, shown, "{name}");
        // And each by the picker's name for it.
        for model in facts.speech.downloaded.known().into_iter().flatten() {
            let definition = models::all_models()
                .into_iter()
                .find(|definition| definition.id.as_str() == model.id)
                .expect("a known model");
            assert_eq!(model.name, definition.name, "{name}");
        }
        assert_eq!(
            facts.speech.model.id,
            ui.settings.transcription.model.as_str(),
            "{name}"
        );
    }
}

#[test]
fn the_notes_writer_is_the_one_written_by_shows_in_use() {
    for (name, ui) in windows() {
        let notes = facts(&ui).notes;
        // Meetings › Settings shows Summaries as passed over exactly while
        // a writer is in use.
        assert_eq!(notes.writer.is_some(), writer_chosen(&ui), "{name}");
        if let Some(writer) = &notes.writer {
            assert_eq!(
                Some(writer.id.as_str()),
                ui.settings.meetings.notes.writer.as_deref(),
                "{name}"
            );
        }
        assert_eq!(notes.meetings, ui.settings.meetings.enabled, "{name}");
    }
}

#[test]
fn the_engines_are_the_ones_the_runs_on_rows_name() {
    for (name, ui) in windows() {
        let facts = facts(&ui);
        let settings = &ui.settings;
        let inherited = dictation(settings);

        // Output › AI cleanup: on, and where.
        let on = settings
            .dictation_processing
            .default_mode
            .post_processing
            .enabled;
        match &facts.cleanup {
            Cleanup::On(runs) => {
                assert!(on, "{name}");
                assert_eq!(runs, &Runs::of(&inherited), "{name}");
            }
            Cleanup::Off => assert!(!on, "{name}"),
        }

        // Files › Settings › Runs on: its own, or "Same as dictation" with
        // the engine that resolves to.
        let config = crate::files::worker::Config::from_settings(settings);
        match &facts.files {
            FilesCleanup::Off => assert!(!settings.labs.files, "{name}"),
            FilesCleanup::AsDictation(runs) => {
                assert!(settings.files.cleanup_engine.is_none(), "{name}");
                assert_eq!(runs, &Runs::of(&inherited), "{name}");
                assert_same_label(name, runs, &inherited);
            }
            FilesCleanup::Own(runs) => {
                let own = config.cleanup_engine(None);
                assert_eq!(runs, &Runs::of(&own), "{name}");
                assert_same_label(name, runs, &own);
            }
        }

        // Assistant › Settings › Runs on, and what a question is sent to.
        let asked = super::engine(settings, None, None);
        assert_eq!(facts.assistant, Runs::of(&asked), "{name}");
        if settings.assistant.engine.is_none() {
            assert_eq!(facts.assistant, Runs::of(&inherited), "{name}");
        }
    }
}

/// The window's label for `engine` names the same model the fact does.
fn assert_same_label(name: &str, runs: &Runs, engine: &Engine) {
    let label = engine_label(engine);
    assert!(
        runs.model.is_empty() || label.contains(&runs.model),
        "{name}: {label:?} names {:?}",
        runs.model
    );
}

/// The window's shortcut rows by the settings key each saves to.
fn rows() -> Vec<(&'static str, ShortcutTarget)> {
    #[allow(unused_mut)]
    let mut rows = vec![
        ("dictation_hotkey", ShortcutTarget::Dictation),
        ("paste_last_hotkey", ShortcutTarget::PasteLast),
        ("raw_dictation_hotkey", ShortcutTarget::RawDictation),
        ("cancel_hotkey", ShortcutTarget::Cancel),
    ];
    #[cfg(target_os = "macos")]
    rows.extend([
        ("second_dictation_hotkey", ShortcutTarget::SecondDictation),
        ("vocabulary_add_hotkey", ShortcutTarget::VocabularyAdd),
        ("stop_hotkey", ShortcutTarget::Stop),
        ("screenshots.hotkey", ShortcutTarget::Screenshot),
        (
            "screenshots.record_hotkey",
            ShortcutTarget::ScreenshotRecord,
        ),
    ]);
    rows
}

#[test]
fn a_shortcut_reads_as_its_rows_keycaps() {
    for (name, ui) in windows() {
        let facts = facts(&ui);
        let rows = rows();
        let mut held = 0;
        for shortcut in &facts.shortcuts {
            let Some((_, target)) = rows.iter().find(|(key, _)| *key == shortcut.key) else {
                continue;
            };
            let row = binding(&ui.settings, *target)
                .unwrap_or_else(|| panic!("{name}: {} is set", shortcut.key))
                .keycaps();
            assert_eq!(
                shortcut.keys.replace('+', ""),
                row.concat().replace('+', ""),
                "{name}: {}",
                shortcut.key
            );
            held += 1;
        }
        assert!(held >= 2, "{name}: {:?}", facts.shortcuts);
        // A row with no key isn't among the facts.
        for (key, target) in &rows {
            if binding(&ui.settings, *target).is_none() {
                assert!(
                    facts.shortcuts.iter().all(|shortcut| shortcut.key != *key),
                    "{name}: {key}"
                );
            }
        }
    }
}
