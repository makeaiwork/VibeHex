//! Fixtures for snapshots and tests: the named starting states `--scene`
//! opens.

use super::*;

mod fixtures;
pub(super) use fixtures::{fixture, mac_fixture, portal_fixture};

/// A named starting state for `--scene`.
pub(super) type Scene = (&'static str, fn(&mut Ui));

pub(super) const SCENES: &[Scene] = &[
    // The running service's real state, for headless snapshots and scripts
    // (`--snap`): waits for its first snapshot instead of drawing
    // "Connecting…" at virtual time zero.
    ("live", |ui| {
        // Reads the saved settings and the running Hex; writes neither.
        let settings = AppSettings::read().unwrap_or_default();
        crate::desktop::i18n::apply(settings.ui_language.as_deref());
        *ui = Ui::with(settings, Service::Fixture(Vec::new()));
        let mut backend = Backend::connect(Default::default(), Arc::new(|| {}));
        let deadline = Instant::now() + Duration::from_secs(5);
        while backend.state().is_none() && backend.error().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
            backend.refresh();
        }
        ui.service = Service::Live(Box::new(backend));
        poll(ui, 0.0, &mut Tx::default());
    }),
    ("portal", |ui| *ui = portal_fixture()),
    ("portal-mouse", |ui| {
        *ui = portal_fixture();
        ui.settings.dictation_hotkey = HotkeyBinding {
            modifiers: Default::default(),
            key: crate::app_settings::HotkeyKey::mouse(3),
        };
        if let Some(view) = &mut ui.view {
            view.listener.shortcuts =
                DesktopShortcuts::Portal(vec![crate::desktop::host::DesktopGrantedShortcut {
                    name: "Dictation".into(),
                    trigger: None,
                    unconfirmed: false,
                }]);
        }
    }),
    ("commands", |ui| *ui = fixture(DesktopShortcuts::Commands)),
    ("disabled", |ui| {
        *ui = portal_fixture();
        if let Some(view) = &mut ui.view {
            view.listener.running = false;
            view.listener.status = "Disabled".into();
        }
    }),
    ("capturing", |ui| {
        *ui = portal_fixture();
        ui.capture = Capture::Listening {
            target: ShortcutTarget::Dictation,
            message: None,
            held: Default::default(),
        };
    }),
    ("chinese", |ui| {
        *ui = portal_fixture();
        ui.settings.ui_language = Some("zh".into());
        crate::desktop::i18n::apply(Some("zh"));
    }),
    ("menus", |ui| {
        *ui = portal_fixture();
        ui.open = Some(Menu::Microphone);
        ui.microphones = Ok(crate::audio::InputDevices {
            names: vec!["Built-in Audio Analog Stereo".into(), "Shure MV7".into()],
            system_default: Some("Shure MV7".into()),
            default_is_hex_microphone: false,
            ..Default::default()
        });
    }),
    ("history", |ui| {
        *ui = portal_fixture();
        ui.history = history::HistoryPane::fixture();
        ui.pane = Pane::History;
    }),
    ("about", |ui| {
        *ui = mac_fixture();
        ui.pane = Pane::About;
    }),
    ("history-menu", |ui| {
        *ui = portal_fixture();
        ui.history = history::HistoryPane::fixture();
        ui.pane = Pane::History;
        ui.open = Some(Menu::HistoryRetention);
    }),
    ("history-clear", |ui| {
        *ui = portal_fixture();
        ui.history = history::HistoryPane::fixture();
        ui.history.clear_armed = true;
        ui.pane = Pane::History;
    }),
    // Labs › Dictation stats: the Stats button with its dialog open.
    ("history-stats", |ui| {
        *ui = portal_fixture();
        ui.history = history::HistoryPane::fixture();
        ui.history.stats = Some(history::HistoryPane::fixture_stats());
        ui.history.stats_open = true;
        ui.settings.labs.history_stats = true;
        crate::labs::apply(&ui.settings.labs);
        ui.pane = Pane::History;
    }),
    // Labs › Screenshots' Captures: a shot, a recording with its words, and
    // an older shot; and before any.
    #[cfg(hex_capture)]
    ("captures", |ui| {
        *ui = portal_fixture();
        ui.settings.labs.screenshots = true;
        ui.captures = captures::CapturesPane::fixture();
        ui.pane = Pane::Captures;
    }),
    #[cfg(hex_capture)]
    ("captures-empty", |ui| {
        *ui = portal_fixture();
        ui.settings.labs.screenshots = true;
        ui.captures = captures::CapturesPane::fixture_empty();
        ui.pane = Pane::Captures;
    }),
    ("history-empty", |ui| {
        *ui = portal_fixture();
        ui.history = history::HistoryPane::fixture_empty();
        ui.pane = Pane::History;
    }),
    ("output", |ui| {
        *ui = portal_fixture();
        output::fixture(ui);
        ui.pane = Pane::Output;
    }),
    ("output-open", |ui| {
        *ui = portal_fixture();
        output::fixture(ui);
        ui.output.expanded = Some(0);
        ui.output.prompt_copied = true;
        ui.pane = Pane::Output;
    }),
    ("output-trust", |ui| {
        *ui = portal_fixture();
        output::trust_fixture(ui);
    }),
    ("output-ai-cleanup", |ui| {
        *ui = portal_fixture();
        output::fixture(ui);
        output::ai_cleanup_fixture(ui);
        ui.pane = Pane::Output;
    }),
    ("output-ai-cleanup-codex", |ui| {
        *ui = portal_fixture();
        output::fixture(ui);
        output::ai_cleanup_codex_fixture(ui);
        ui.pane = Pane::Output;
    }),
    ("output-ai-cleanup-local", |ui| {
        *ui = portal_fixture();
        output::fixture(ui);
        output::ai_cleanup_local_fixture(ui);
        ui.pane = Pane::Output;
    }),
    ("output-empty", |ui| {
        *ui = portal_fixture();
        ui.pane = Pane::Output;
    }),
    ("modes", |ui| {
        *ui = portal_fixture();
        modes::fixture(ui);
        ui.pane = Pane::Modes;
    }),
    ("modes-opencode-not-running", |ui| {
        *ui = portal_fixture();
        modes::fixture(ui);
        ui.models = models::Models::fixture_not_running();
        ui.settings.dictation_processing.modes[0]
            .post_processing
            .enabled = false;
        ui.pane = Pane::Modes;
    }),
    ("modes-global", |ui| {
        *ui = portal_fixture();
        modes::fixture(ui);
        ui.modes.select(modes::Selection::Default);
        ui.pane = Pane::Modes;
    }),
    ("modes-picker", |ui| {
        *ui = portal_fixture();
        modes::fixture(ui);
        ui.modes.application_picker_open = true;
        ui.pane = Pane::Modes;
    }),
    ("modes-transformation-menu", |ui| {
        *ui = portal_fixture();
        modes::fixture(ui);
        ui.open = Some(Menu::Transformations);
        ui.pane = Pane::Modes;
    }),
    ("picker", |ui| {
        *ui = portal_fixture();
        picker::fixture(ui, None);
    }),
    ("picker-downloading", |ui| {
        *ui = portal_fixture();
        picker::fixture(ui, Some(TranscriptionModelId::CohereTranscribe));
    }),
    // Labs › Parakeet Ultra: its card's details, credit, and published
    // benchmarks (lists Ultra only where the saved settings turn it on).
    ("picker-parakeet-ultra", |ui| {
        *ui = portal_fixture();
        picker::parakeet_ultra_fixture(ui);
    }),
    // Every model, with the full list's Add model file….
    ("picker-custom-models", |ui| {
        *ui = portal_fixture();
        picker::all_fixture(ui);
    }),
    // Found on this computer: Parakeet v3's exact file and an untested Whisper.
    ("picker-found", |ui| {
        *ui = portal_fixture();
        ui.found.models = found::fixture();
        picker::all_fixture(ui);
    }),
    // Japanese saved: Japanese models compared, SenseVoice downloading.
    ("picker-japanese", |ui| {
        *ui = portal_fixture();
        picker::japanese_fixture(ui, Some(TranscriptionModelId::SenseVoiceSmall));
    }),
    // Every model: the English-only ones fold away and switch the language.
    ("picker-japanese-all", |ui| {
        *ui = portal_fixture();
        picker::japanese_fixture(ui, None);
        (ui.picker.view, ui.picker.others_open) = (picker::View::All, true);
    }),
    // Parakeet chosen over Japanese: Settings names the language change.
    ("language-fallback", |ui| {
        *ui = portal_fixture();
        ui.picker.fallback = Some(picker::LanguageFallback {
            model: TranscriptionModelId::ParakeetUnifiedEnglish,
            from: "ja".into(),
        });
    }),
    // Whisper's languages open below the model.
    ("language-menu", |ui| {
        *ui = portal_fixture();
        ui.settings.transcription.model = TranscriptionModelId::WhisperLargeV3Turbo;
        ui.picker.installed = Some(vec![TranscriptionModelId::WhisperLargeV3Turbo]);
        ui.open = Some(Menu::Pick("transcription-language"));
    }),
    ("shortcut-mode", shortcut_mode::scene_modifier),
    ("shortcut-mode-key", shortcut_mode::scene_key),
    ("shortcut-mode-chord", shortcut_mode::scene_chord),
    ("shortcut-mode-motion", shortcut_mode::scene_motion),
    // The Shortcut row with a modifier-only shortcut, Left Option, beside
    // its side picker (macOS), in each face of the recorder.
    ("shortcut-row", |ui| shortcut_row(ui, Capture::Idle)),
    ("shortcut-row-right", |ui| {
        shortcut_row(ui, Capture::Idle);
        ui.settings.dictation_hotkey.modifiers.option = Some(ModifierSide::Right);
    }),
    // Two sided modifiers have no side picker, so their keycaps name the side.
    ("shortcut-row-sided-chord", |ui| {
        shortcut_row(ui, Capture::Idle);
        ui.settings.dictation_hotkey.modifiers.command = Some(ModifierSide::Right);
    }),
    ("shortcut-row-recording", |ui| {
        shortcut_row(ui, listening(None, HotkeyModifiers::default()));
    }),
    ("shortcut-row-held", |ui| {
        shortcut_row(ui, listening(None, HotkeyModifiers::option_command()));
    }),
    ("shortcut-row-rejected", |ui| {
        let message = crate::desktop::i18n::tr("Add a modifier");
        shortcut_row(ui, listening(Some(message), HotkeyModifiers::default()));
    }),
    ("shortcut-row-saved", |ui| {
        shortcut_row(
            ui,
            Capture::Saved {
                target: ShortcutTarget::Dictation,
                until: None,
            },
        );
    }),
    // The first run on Linux (Windows is the same window): the model step
    // as it opens on an English system (nothing downloads), with Russian
    // chosen there, with the pick downloading, failed, and with models
    // found here; then the demo while the model downloads, waiting for it,
    // the practice game, and Hex is ready.
    ("onboarding", |ui| {
        first_run(ui, FirstRun::Choosing, Page::Download)
    }),
    ("onboarding-choice-russian", |ui| {
        first_run(ui, FirstRun::Choosing, Page::Download);
        onboarding_kit::choice::choose_language(ui, "ru");
    }),
    // Russian with a file found here that speaks it: its four offers and
    // the found row, the most rows the step shows.
    ("onboarding-choice-russian-found", found::russian),
    // The same two in Russian and German, the longest lines the page has.
    ("onboarding-choice-in-russian", |ui| {
        first_run_in(ui, "ru");
        onboarding_kit::choice::choose_language(ui, "ru");
    }),
    ("onboarding-choice-in-german", |ui| first_run_in(ui, "de")),
    // Back on the list while the pick downloads: its progress, and Cancel.
    ("onboarding-choice-downloading", |ui| {
        first_run(ui, FirstRun::Downloading, Page::Demo);
        onboarding_kit::live::back_to_models(ui);
        onboarding_kit::live::fixture_resume(ui, Page::Download);
    }),
    ("onboarding-choice-failed", |ui| {
        first_run(ui, FirstRun::Failed, Page::Download)
    }),
    // The failure where the practice waits for the model, with its ways on.
    ("onboarding-download-failed", |ui| {
        first_run(ui, FirstRun::Failed, Page::Waiting)
    }),
    // Continue without a model from the start, then Settings with no model.
    ("onboarding-no-model", |ui| {
        first_run(ui, FirstRun::Choosing, Page::Download);
        onboarding_kit::live::continue_without_model(ui);
    }),
    // "All models…" from the model step: the picker for the language chosen
    // there, where a model is only chosen for the step.
    ("picker-first-run", |ui| {
        first_run(ui, FirstRun::Choosing, Page::Download);
        onboarding_kit::choice::choose_language(ui, "ru");
        onboarding_kit::choice::all_models(ui);
    }),
    ("onboarding-model-found", found::listed),
    ("onboarding-model-found-downloading", found::loading),
    ("onboarding-model-found-recommended", found::present),
    ("onboarding-model-found-back", found::chosen),
    ("onboarding-demo", |ui| {
        first_run(ui, FirstRun::Downloading, Page::Demo)
    }),
    ("onboarding-waiting", |ui| {
        first_run(ui, FirstRun::Downloading, Page::Waiting)
    }),
    // GNOME and KDE on Wayland: why the desktop asks before the first paste.
    ("onboarding-paste", |ui| {
        first_run(ui, FirstRun::PasteAccess, Page::Permissions)
    }),
    ("onboarding-try", |ui| {
        first_run(ui, FirstRun::Ready, Page::Try)
    }),
    ("onboarding-ready", |ui| {
        first_run(ui, FirstRun::Ready, Page::Ready)
    }),
    // Windows with its microphone privacy switch off.
    ("windows-onboarding-microphone", |ui| {
        *ui = fixture(DesktopShortcuts::Registered);
        ui.platform = platform::PlatformState::preview(platform::Capabilities::WINDOWS);
        ui.host = platform::PlatformHost::Fixture(Vec::new());
        ui.platform.permissions.microphone =
            crate::desktop::onboarding::PermissionState::NeedsSettings;
        onboarding::fixture(ui, crate::desktop::onboarding::Step::Setup, true);
        onboarding_kit::live::fixture(ui, Page::Permissions);
    }),
    // macOS and Windows surfaces, from preview data (see `platform`).
    ("mac-settings", |ui| *ui = mac_fixture()),
    // The microphone row following a recorded voice (`--video`), and with
    // the microphone released while idle, when no audio arrives.
    ("mac-settings-mic-level", |ui| {
        *ui = mac_fixture();
        ui.mic_level = general::MicLevel::playing(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/vocabulary/audio/term-opencode-2.wav"
        )));
    }),
    ("mac-settings-mic-released", |ui| {
        *ui = mac_fixture();
        ui.settings.release_microphone_while_idle = true;
        ui.mic_level = general::MicLevel::closed();
    }),
    ("mac-settings-ducking", |ui| {
        *ui = mac_fixture();
        ui.settings.recording_audio_behavior = crate::app_settings::RecordingAudioBehavior::Duck;
    }),
    ("mac-input-source-models", |ui| {
        use crate::app_settings::KeyboardModelChoice;
        *ui = mac_fixture();
        ui.advanced = vec![Pane::General];
        ui.settings.keyboard_model_switching.enabled = true;
        ui.settings.keyboard_model_switching.mappings = [
            (
                "com.apple.keylayout.ABC",
                TranscriptionModelId::ParakeetUnifiedEnglish,
                "en",
            ),
            (
                "com.apple.keylayout.RussianWin",
                TranscriptionModelId::ParakeetV3,
                "ru",
            ),
        ]
        .into_iter()
        .map(|(id, model, language)| {
            (
                id.into(),
                KeyboardModelChoice {
                    model,
                    language: language.into(),
                },
            )
        })
        .collect();
        ui.platform.input_sources = [
            ("com.apple.keylayout.ABC", "ABC", "en"),
            ("com.apple.keylayout.RussianWin", "Russian - PC", "ru"),
            ("com.apple.keylayout.Spanish", "Spanish", "es"),
        ]
        .into_iter()
        .map(
            |(id, name, language)| crate::transcription::keyboard_model_switching::InputSource {
                id: id.into(),
                name: name.into(),
                languages: vec![language.into()],
            },
        )
        .collect();
        ui.picker.installed = Some(vec![
            TranscriptionModelId::ParakeetUnifiedEnglish,
            TranscriptionModelId::ParakeetV3,
        ]);
    }),
    // The sound sets sheet at rest, then Wood's dictation recording.
    ("sound-sets", |ui| {
        *ui = mac_fixture();
        sound_sets::fixture(ui, None);
    }),
    ("sound-sets-playing", |ui| {
        *ui = mac_fixture();
        sound_sets::fixture(
            ui,
            Some((
                crate::feedback::SoundSet::Wood,
                crate::feedback::Audition::Dictation,
                900.0,
            )),
        );
    }),
    ("mac-microphone-missing", |ui| {
        *ui = mac_fixture();
        ui.settings.microphone = Some("Shure MV7".into());
        ui.open = Some(Menu::Microphone);
    }),
    ("mac-microphone-lid-closed", |ui| {
        *ui = mac_fixture();
        if let Ok(devices) = &mut ui.microphones {
            devices.system_default = Some("Studio Display Microphone".into());
            devices.off_with_lid_closed = Some("MacBook Pro Microphone".into());
            devices.default_is_off_with_lid_closed = true;
        }
        ui.open = Some(Menu::Microphone);
    }),
    // System default passes over the Bluetooth default for the built-in mic.
    ("mac-microphone-bluetooth", |ui| {
        *ui = mac_fixture();
        if let Ok(devices) = &mut ui.microphones {
            devices.system_default = Some("MacBook Pro Microphone".into());
            devices.bluetooth_default_passed_over = Some("Kit Pods".into());
        }
        ui.advanced = vec![Pane::General];
    }),
    ("mac-settings-vocabulary", |ui| {
        *ui = mac_fixture();
        vocabulary::fixture(ui, true);
    }),
    ("mac-settings-vocabulary-text", |ui| {
        *ui = mac_fixture();
        vocabulary::fixture(ui, true);
        vocabulary::text_fixture(ui);
    }),
    ("mac-settings-vocabulary-empty", |ui| {
        *ui = mac_fixture();
        vocabulary::fixture(ui, false);
    }),
    // Cohere ignores the list: the row and the sheet say so.
    ("vocabulary-unused", |ui| {
        *ui = portal_fixture();
        ui.settings.transcription.model = TranscriptionModelId::CohereTranscribe;
        ui.picker.installed = Some(vec![TranscriptionModelId::CohereTranscribe]);
        vocabulary::fixture(ui, true);
    }),
    // Whisper reads only the terms that fit its prompt: the sheet says how many.
    ("vocabulary-budget", |ui| {
        *ui = portal_fixture();
        ui.settings.transcription.model = TranscriptionModelId::WhisperLargeV3Turbo;
        ui.picker.installed = Some(vec![TranscriptionModelId::WhisperLargeV3Turbo]);
        vocabulary::fixture(ui, true);
        let entries: Vec<_> = (1..=52)
            .map(|n| crate::dictation::vocabulary::VocabularyEntry {
                term: format!("Project codename {n}"),
                sounds_like: Vec::new(),
            })
            .collect();
        ui.settings.vocabulary = entries;
        ui.vocabulary.reload(&ui.settings.vocabulary);
    }),
    ("mac-settings-recording", |ui| {
        *ui = mac_fixture();
        let held = crate::app_settings::HotkeyModifiers {
            option: Some(crate::app_settings::ModifierSide::Either),
            command: Some(crate::app_settings::ModifierSide::Either),
            ..Default::default()
        };
        ui.capture = Capture::Listening {
            target: ShortcutTarget::Dictation,
            message: None,
            held,
        };
    }),
    ("mac-settings-permissions", |ui| {
        platform_settings::mac_permissions_fixture(ui, false)
    }),
    ("mac-settings-permissions-stale", |ui| {
        platform_settings::mac_permissions_fixture(ui, true)
    }),
    ("mac-settings-pause-browser", |ui| {
        *ui = mac_fixture();
        ui.settings.recording_audio_behavior =
            crate::app_settings::RecordingAudioBehavior::PauseMedia;
        ui.settings.pause_browser_media = true;
        ui.platform.system_audio = false;
    }),
    ("mac-settings-advanced", |ui| {
        *ui = mac_fixture();
        ui.advanced = vec![Pane::General];
    }),
    ("mac-settings-dev-channel", |ui| {
        *ui = mac_fixture();
        ui.settings.updates.channel = Some(crate::app_settings::UpdateChannel::Dev);
    }),
    // A local build: the badge, the quiet status line, no channel row.
    ("mac-local-build", |ui| {
        *ui = mac_fixture();
        ui.build_preview = Some(BuildKind::Local);
        if let Some(view) = &mut ui.view {
            view.update_status = DesktopUpdateStatus::SourceBuild;
        }
    }),
    ("mac-test-build", |ui| {
        *ui = mac_fixture();
        ui.build_preview = Some(BuildKind::Test);
    }),
    ("mac-update-ready", |ui| {
        *ui = mac_fixture();
        if let Some(view) = &mut ui.view {
            view.update_status = DesktopUpdateStatus::ReadyToRestart;
        }
    }),
    ("mac-update-sheet", |ui| {
        *ui = mac_fixture();
        super::update_sheet::fixture(ui);
    }),
    ("mac-update-checking", |ui| {
        *ui = mac_fixture();
        if let Some(view) = &mut ui.view {
            view.update_status = DesktopUpdateStatus::Checking;
        }
    }),
    ("mac-update-downloading", |ui| {
        *ui = mac_fixture();
        if let Some(view) = &mut ui.view {
            view.update_status = DesktopUpdateStatus::Downloading;
        }
    }),
    ("windows-settings", |ui| {
        *ui = fixture(DesktopShortcuts::Registered);
        ui.platform = platform::PlatformState::preview(platform::Capabilities::WINDOWS);
        ui.host = platform::PlatformHost::Fixture(Vec::new());
        ui.settings.paste_with_shift = false;
    }),
    ("meetings", |ui| {
        *ui = mac_fixture();
        ui.pane = Pane::Meetings;
    }),
    ("meetings-settings", |ui| {
        *ui = mac_fixture();
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
    }),
    ("meetings-notes", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
    }),
    ("meetings-questions-menu", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.settings.meetings.questions.engine = Some("codex:gpt-6.1-sol".into());
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
        ui.open = Some(Menu::Pick("meeting-questions-engine"));
    }),
    ("meetings-notes-unavailable", |ui| {
        *ui = mac_fixture();
        ui.settings.labs.ai_cleanup_server = true;
        ui.platform.notes = super::platform::NotesAvailability::NotEnabled;
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
    }),
    ("meetings-notes-writer", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
        ui.open = Some(Menu::Pick("meeting-notes-writer"));
    }),
    ("meetings-codex-notes", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        if let Some(meeting) = ui
            .meetings
            .meetings
            .iter_mut()
            .find(|meeting| meeting.id == "preview-notes")
        {
            meeting.summary = Some(
                concat!(
                    "**Summary:** one login for every Anomaly Arcade game; the build screen needs a proposal first.\n\n",
                    "**Decisions:**\n- Ship the shared login before new games (00:12)\n\n",
                    "**Action items:**\n- [ ] Draft the build screen proposal. Who: Them. By: Friday"
                )
                .into(),
            );
            meeting.notes_by = Some(
                "Written by Qwen3 4B, because the chosen writer failed: GPT-6-Luna: your ChatGPT plan's Codex limit is reached".into(),
            );
        }
        ui.meetings.select("preview-notes".into());
        ui.pane = Pane::Meetings;
    }),
    ("meetings-requests", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.select("preview-notes".into());
        ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Requests);
        super::meeting_tabs::requests::fixture(ui, "preview-notes");
        ui.pane = Pane::Meetings;
    }),
    ("meetings-transcript-ask", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.select("preview-notes".into());
        ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Transcript);
        super::meeting_tabs::requests::fixture(ui, "preview-notes");
        ui.pane = Pane::Meetings;
    }),
    ("meetings-recent", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.select("preview-notes".into());
        ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Transcript);
        super::meeting_tabs::requests::fixture(ui, "preview-notes");
        ui.files.recent = vec![
            "Who owns the next step?".into(),
            "What did we agree on about the launch date?".into(),
        ];
        ui.open = Some(super::Menu::Pick("meeting-recent"));
        ui.pane = Pane::Meetings;
    }),
    ("meetings-requests-sessions", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.select("preview-notes".into());
        ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Requests);
        super::meeting_tabs::requests::fixture(ui, "preview-notes");
        ui.open = Some(super::Menu::Pick("meeting-sessions"));
        ui.pane = Pane::Meetings;
    }),
    // A question that read other recordings, and one waiting for leave.
    ("meetings-requests-linked", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.select("preview-notes".into());
        ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Requests);
        super::meeting_tabs::linked::fixture(ui, "preview-notes", false);
        ui.pane = Pane::Meetings;
    }),
    ("meetings-requests-consent", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.select("preview-notes".into());
        ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Requests);
        super::meeting_tabs::linked::fixture(ui, "preview-notes", true);
        ui.pane = Pane::Meetings;
    }),
    ("meetings-prompt", |ui| {
        *ui = mac_fixture();
        meetings_on_codex(ui);
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
        super::meeting_notes::open_prompt(ui);
    }),
    // Summaries' picker open: Off, On this Mac, OpenCode, each saying what
    // it does; then with Apple Intelligence off, On this Mac kept, disabled,
    // with the fix.
    ("meetings-summaries", |ui| {
        *ui = mac_fixture();
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
        ui.open = Some(Menu::Pick("meeting-summaries"));
    }),
    ("meetings-summaries-unavailable", |ui| {
        *ui = mac_fixture();
        ui.platform.notes = platform::NotesAvailability::NotEnabled;
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
        ui.open = Some(Menu::Pick("meeting-summaries"));
    }),
    // Summaries set to OpenCode: its model nested under it, then with
    // OpenCode not running, the saved model kept.
    ("meetings-summary-model", |ui| {
        summary_model_scene(ui, models::Models::fixture())
    }),
    ("meetings-summary-model-not-running", |ui| {
        summary_model_scene(ui, models::Models::fixture_not_running())
    }),
    // System Audio not yet allowed: tap Allow to wait, again to grant.
    ("meetings-allow", |ui| {
        *ui = mac_fixture();
        ui.platform.system_audio = false;
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
    }),
    ("meetings-calendar", |ui| {
        *ui = mac_fixture();
        ui.settings.meetings.calendar.connected = true;
        ui.settings.meetings.calendar.excluded_calendars = vec!["birthdays".into()];
        ui.platform.calendar = platform::CalendarView::preview();
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
    }),
    ("meetings-calendar-denied", |ui| {
        *ui = mac_fixture();
        ui.platform.calendar.access = crate::calendar::CalendarAccess::Denied;
        ui.meetings.view = meetings::View::Settings;
        ui.pane = Pane::Meetings;
    }),
    ("files", |ui| files_scene(ui, files::FilesPane::fixture())),
    ("assistant", assistant::fixture::conversation),
    ("assistant-answering", assistant::fixture::answering),
    ("assistant-places", assistant::fixture::places),
    ("assistant-place-opened", assistant::fixture::place_opened),
    ("assistant-lab-opened", assistant::fixture::lab_opened),
    ("assistant-start", assistant::fixture::start),
    ("assistant-start-russian", assistant::fixture::start_russian),
    ("assistant-connect", assistant::fixture::connect),
    ("assistant-setup-unreachable", assistant::connect::scene),
    ("assistant-settings", assistant::fixture::settings),
    ("assistant-consent", assistant::fixture::consent),
    ("assistant-steps", assistant::fixture::steps),
    ("assistant-agent-consent", assistant::fixture::agent_consent),
    ("assistant-recordings", assistant::fixture::recordings),
    ("assistant-steps-open", assistant::fixture::steps_open),
    ("assistant-knew", assistant::fixture::knew),
    ("assistant-web", assistant::fixture::web_answer),
    ("assistant-web-off", assistant::fixture::web_off),
    ("assistant-web-held", assistant::fixture::web_held),
    ("assistant-web-answering", assistant::fixture::web_answering),
    ("assistant-cards", assistant::fixture_cards::cards),
    ("assistant-cards-more", assistant::fixture_cards::cards_more),
    ("assistant-cards-held", assistant::fixture_cards::cards_held),
    (
        "assistant-cards-prompt",
        assistant::fixture_cards::cards_prompt,
    ),
    (
        "assistant-cards-prompt-edit",
        assistant::fixture_cards::cards_prompt_edit,
    ),
    ("assistant-applied", assistant::fixture_cards::applied),
    ("assistant-whats-new", assistant::fixture_cards::whats_new),
    ("assistant-updated", assistant::fixture_cards::updated),
    ("assistant-marked", assistant::fixture_cards::marked),
    ("assistant-marked-rail", assistant::fixture_cards::rail),
    ("assistant-vocab-card", assistant::fixture_cards::vocab_card),
    (
        "assistant-vocab-applied",
        assistant::fixture_cards::vocab_applied,
    ),
    ("assistant-vocab-edit", assistant::fixture_cards::vocab_edit),
    (
        "assistant-change-both",
        assistant::fixture_cards::change_both,
    ),
    (
        "assistant-change-coming",
        assistant::fixture_cards::change_coming,
    ),
    ("assistant-setup-ask", assistant::setup_fixture::setup_ask),
    (
        "assistant-setup-follow",
        assistant::setup_fixture::setup_follow,
    ),
    (
        "assistant-setup-follow-ru",
        assistant::setup_fixture::setup_follow_ru,
    ),
    ("assistant-setup-plan", assistant::setup_fixture::setup_plan),
    ("assistant-setup-kept", assistant::setup_fixture::setup_kept),
    (
        "assistant-setup-result",
        assistant::setup_fixture::setup_result,
    ),
    (
        "assistant-setup-preset",
        assistant::setup_fixture::setup_preset,
    ),
    (
        "assistant-setup-preset-shortcuts",
        assistant::setup_fixture::setup_preset_shortcuts,
    ),
    (
        "assistant-setup-preset-shortcuts-recording",
        assistant::keys::preset_recording,
    ),
    (
        "assistant-setup-preset-shortcuts-recorded",
        assistant::keys::preset_recorded,
    ),
    ("assistant-shortcuts-card", assistant::keys::answer),
    (
        "assistant-setup-preset-terms",
        assistant::terms_fixture::preset_terms_open,
    ),
    ("assistant-plan-terms", assistant::terms_fixture::plan_terms),
    (
        "assistant-plan-terms-open",
        assistant::terms_fixture::plan_terms_open,
    ),
    (
        "assistant-setup-returning",
        assistant::setup_fixture::setup_returning,
    ),
    (
        "assistant-setup-returning-plan",
        assistant::setup_fixture::setup_returning_plan,
    ),
    (
        "assistant-long-follow-german",
        assistant::long_fixture::follow_german,
    ),
    (
        "assistant-long-plan-russian",
        assistant::long_fixture::plan_russian,
    ),
    (
        "assistant-long-choice-russian",
        assistant::long_fixture::choice_russian,
    ),
    ("assistant-long-values", assistant::long_fixture::values),
    (
        "assistant-long-values-open",
        assistant::long_fixture::values_open,
    ),
    ("meetings-cited", assistant::fixture::meeting_cited),
    // Opened from a meeting's Requests: Back to Requests.
    ("meetings-cited-requests", |ui| {
        assistant::fixture::meeting_cited(ui);
        if let Some(from) = ui.came_from.as_mut() {
            from.origin = assistant::jump::Origin::Requests {
                kind: crate::assistant::recordings::Kind::Meeting,
                id: "preview-arcade".into(),
                session: 3,
            };
        }
    }),
    (
        "meetings-cited-notes",
        assistant::fixture::meeting_notes_cited,
    ),
    (
        "meetings-cited-long",
        assistant::fixture::meeting_cited_long,
    ),
    ("files-cited", assistant::fixture::file_cited),
    (
        "files-cited-listening",
        assistant::fixture::file_cited_listening,
    ),
    ("history-cited", assistant::fixture::history_cited),
    ("files-playback", |ui| files_playback(ui, true)),
    ("files-playback-folded", |ui| files_playback(ui, false)),
    ("files-playback-cleanup", |ui| {
        files_playback(ui, true);
        ui.files.tab = Some(files::Tab::Version(2));
    }),
    ("meetings-playback", |ui| meetings_playback(ui, true)),
    ("meetings-player", |ui| {
        meetings_playback(ui, false);
        ui.settings.labs.ai_cleanup_server = false;
    }),
    ("meetings-playback-folded", |ui| {
        meetings_playback(ui, false)
    }),
    ("sidebar-foldable", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.labs.collapsible_sidebar = true;
        ui.sidebar_tip = Some("sidebar-close");
    }),
    ("sidebar-folded", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.labs.collapsible_sidebar = true;
        ui.settings.labs.sidebar_collapsed = true;
        ui.sidebar_tip = Some("nav-meetings");
    }),
    ("files-working", |ui| {
        files_scene(ui, files::FilesPane::working_fixture())
    }),
    ("files-clean-up", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(4));
    }),
    ("files-version-info", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(5));
        ui.files.hover = Some("info");
    }),
    ("files-edit", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(4));
        let record = ui.files.selected().cloned();
        let text = record.and_then(|record| Some(record.version(4)?.text.clone()));
        // Long enough to scroll inside the field.
        ui.files.edit = rune::TextEdit::new(vec![text.unwrap_or_default(); 3].join("\n\n"));
        ui.files.edit.focused = true;
        ui.files.editing = Some(4);
    }),
    ("files-original", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(1));
    }),
    ("files-summary", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(3));
    }),
    ("files-session", |ui| {
        files_scene(ui, files::FilesPane::session_fixture());
        let server = &mut ui.settings.dictation_processing.server;
        server.runs_on = crate::app_settings::CleanupRunsOn::Codex;
        server.codex_model = "gpt-6-luna".into();
        ui.files.tab = Some(files::Tab::Version(3));
    }),
    ("files-model-menu", |ui| {
        files_scene(ui, files::FilesPane::session_fixture());
        let server = &mut ui.settings.dictation_processing.server;
        server.runs_on = crate::app_settings::CleanupRunsOn::Codex;
        server.codex_model = "gpt-6-luna".into();
        ui.files.tab = Some(files::Tab::Version(3));
        ui.open = Some(Menu::Pick("files-composer-model"));
    }),
    ("files-versions", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(3));
        ui.open = Some(Menu::Pick("files-versions"));
    }),
    ("files-more", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.opener = Some("QuickTime Player".into());
        ui.open = Some(Menu::Pick("files-more"));
    }),
    ("files-again", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.files.tab = Some(files::Tab::Version(1));
        ui.files.again = Some(files::Again {
            record: "preview-interview".into(),
            model: crate::transcription::models::TranscriptionModelId::WhisperLargeV3Turbo,
            name: "Whisper Large v3 Turbo",
            language: "ru".into(),
        });
    }),
    ("files-empty", |ui| {
        files_scene(ui, files::FilesPane::empty_fixture())
    }),
    ("files-settings", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.transcription = crate::transcription::models::TranscriptionSelection {
            model: crate::transcription::models::TranscriptionModelId::WhisperLargeV3Turbo,
            language: "ru".into(),
        };
        ui.settings.labs.ai_cleanup_server = true;
        ui.settings.dictation_processing.server.runs_on = crate::app_settings::CleanupRunsOn::Local;
        ui.files.view = files::View::Settings;
    }),
    ("files-engine-menu", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.labs.ai_cleanup_server = true;
        let server = &mut ui.settings.dictation_processing.server;
        server.runs_on = crate::app_settings::CleanupRunsOn::Codex;
        server.codex_model = "gpt-6-luna".into();
        server.openrouter_model = "google/gemma-3-4b-it".into();
        server.url = "http://127.0.0.1:1234/v1".into();
        server.custom_model = "gemma-4-e4b-it-mlx-4bit".into();
        ui.files.codex_models = files::FilesPane::fixture_codex_models();
        ui.files.view = files::View::Settings;
        ui.open = Some(Menu::Pick("files-request-engine"));
    }),
    // Cleanup on dictation's Codex, its Reasoning following dictation's.
    ("files-settings-codex", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.labs.ai_cleanup_server = true;
        let server = &mut ui.settings.dictation_processing.server;
        server.runs_on = crate::app_settings::CleanupRunsOn::Codex;
        server.codex_model = "gpt-6-luna".into();
        server.codex_effort = "high".into();
        ui.files.codex_models = files::FilesPane::fixture_codex_models();
        ui.files.view = files::View::Settings;
    }),
    ("files-settings-prompt", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.files.use_default_prompt = false;
        ui.files.view = files::View::Settings;
    }),
    ("files-quick", |ui| {
        files_scene(ui, files::FilesPane::fixture());
        ui.settings.transcription = crate::transcription::models::TranscriptionSelection {
            model: crate::transcription::models::TranscriptionModelId::WhisperLargeV3Turbo,
            language: "ru".into(),
        };
        ui.settings.files.quick_prompts[2].prompt = "Only my own tasks, as a checklist.".into();
        ui.files.view = files::View::Settings;
        files::open_quick(ui, 0);
    }),
    ("meetings-off", |ui| {
        *ui = mac_fixture();
        ui.settings.meetings.enabled = false;
        ui.pane = Pane::Meetings;
    }),
    ("meetings-recording", |ui| {
        *ui = mac_fixture();
        ui.meetings.select("preview-live".into());
        ui.pane = Pane::Meetings;
    }),
    ("meetings-actions", |ui| {
        *ui = mac_fixture();
        ui.open = Some(Menu::MeetingActions);
        ui.pane = Pane::Meetings;
    }),
    ("meetings-delete", |ui| {
        *ui = mac_fixture();
        ui.meetings.delete_armed = Some("preview-notes".into());
        ui.pane = Pane::Meetings;
    }),
    ("meetings-rename", |ui| {
        *ui = mac_fixture();
        ui.meetings.renaming = Some("preview-notes".into());
        ui.meetings.rename = TextEdit::new("Arcade launch plan");
        ui.pane = Pane::Meetings;
    }),
    ("meetings-failed", |ui| {
        *ui = mac_fixture();
        ui.meetings.select("preview-failed".into());
        ui.pane = Pane::Meetings;
    }),
    ("meetings-chapters", |ui| {
        *ui = mac_fixture();
        ui.meetings = meetings::MeetingsPane::chapters_fixture();
        ui.pane = Pane::Meetings;
    }),
    ("meetings-empty", |ui| {
        *ui = mac_fixture();
        ui.meetings = meetings::MeetingsPane::default();
        ui.pane = Pane::Meetings;
    }),
    #[cfg(hex_capture)]
    ("mac-scrawl-off", |ui| scrawl_fixture(ui, false, |_| {})),
    #[cfg(hex_capture)]
    ("mac-scrawl", |ui| scrawl_fixture(ui, true, |_| {})),
    #[cfg(hex_capture)]
    ("mac-scrawl-running", |ui| {
        scrawl_fixture(ui, true, |view| view.scrawl_running = true)
    }),
    #[cfg(hex_capture)]
    ("mac-scrawl-permission", |ui| {
        scrawl_fixture(ui, true, |view| view.screen_recording = false)
    }),
    #[cfg(hex_capture)]
    ("mac-scrawl-restart", |ui| {
        scrawl_fixture(ui, true, |view| {
            view.screen_recording = false;
            view.screen_needs_restart = true;
        })
    }),
    ("mac-voice-filter", |ui| {
        *ui = mac_fixture();
        ui.pane = Pane::VoiceFilter;
    }),
    ("mac-voice-filter-muted", |ui| {
        *ui = mac_fixture();
        ui.platform.tuning.mode = crate::audio::voice_filter::VoiceFilterMode::Muted;
        ui.platform.demo = Some(fun::preview_demo(
            false,
            Some(crate::audio::hex_microphone::DemoPlaybackTrack::CallHears),
        ));
        ui.pane = Pane::VoiceFilter;
    }),
    ("mac-voice-filter-driver-missing", |ui| {
        *ui = mac_fixture();
        if let Some(driver) = &mut ui.platform.hex_microphone {
            driver.driver_bundle_installed = false;
            driver.driver_loaded = false;
            driver.installed_version = None;
            driver.active_apps.clear();
        }
        ui.platform.demo = Some(fun::preview_demo(true, None));
        ui.pane = Pane::VoiceFilter;
    }),
    ("voice-filter", |ui| {
        *ui = fixture(DesktopShortcuts::Commands);
        ui.platform = platform::PlatformState::preview(platform::Capabilities::LINUX);
        ui.host = platform::PlatformHost::Fixture(Vec::new());
        ui.settings.experimental.hex_microphone = true;
        ui.pane = Pane::VoiceFilter;
    }),
    ("voice-filter-unavailable", |ui| {
        *ui = fixture(DesktopShortcuts::Commands);
        ui.platform = platform::PlatformState::preview(platform::Capabilities::LINUX);
        ui.host = platform::PlatformHost::Fixture(Vec::new());
        if let Some(source) = &mut ui.platform.hex_microphone {
            source.driver_loaded = false;
            source.action_state = crate::audio::hex_microphone::DriverActionState::Failed(
                "pw-cat was not found; Hex Microphone needs PipeWire".into(),
            );
        }
        ui.settings.experimental.hex_microphone = true;
        ui.pane = Pane::VoiceFilter;
    }),
    ("mac-hud-lab", |ui| {
        *ui = mac_fixture();
        ui.platform.developer = true;
        ui.pane = Pane::HudLab;
    }),
    ("mac-labs", labs_with_some_on),
    // Where hex://labs/vocabulary-biasing lands: its group's chip.
    ("mac-labs-link", |ui| {
        *ui = mac_fixture();
        ui.labs_open = true;
        ui.lab_store.group = Some(crate::labs::Feature::VocabularyBiasing.group());
    }),
    ("mac-labs-on", |ui| {
        labs_with_some_on(ui);
        ui.lab_store.tab = labs::Tab::On;
    }),
    ("mac-labs-category", |ui| {
        labs_with_some_on(ui);
        ui.lab_store.group = Some(crate::labs::Group::Shortcuts);
    }),
    ("mac-labs-search", |ui| {
        labs_with_some_on(ui);
        ui.lab_store.search = rune::TextEdit::new("paste".to_string());
    }),
    // A lab's own settings on its card while it's on.
    ("mac-labs-settings", |ui| {
        labs_with_some_on(ui);
        ui.settings.labs.unload_idle = true;
        ui.lab_store.group = Some(crate::labs::Feature::UnloadIdle.group());
    }),
    // Labs › Screenshots' card while on (macOS only: elsewhere the lab has
    // no row).
    ("mac-labs-screenshots", |ui| {
        labs_with_some_on(ui);
        ui.settings.labs.screenshots = true;
        ui.lab_store.group = Some(crate::labs::Feature::Screenshots.group());
    }),
    // Labs on Windows and Linux: only their OS's labs, groups holding a
    // lab that's on open.
    ("windows-labs", |ui| {
        *ui = fixture(DesktopShortcuts::Registered);
        ui.platform = platform::PlatformState::preview(platform::Capabilities::WINDOWS);
        ui.host = platform::PlatformHost::Fixture(Vec::new());
        ui.settings.paste_with_shift = true;
        ui.labs_open = true;
    }),
    ("linux-labs", |ui| {
        *ui = portal_fixture();
        ui.platform.caps = platform::Capabilities::LINUX;
        ui.settings.labs.skip_pauses = true;
        ui.settings.labs.typed_text = true;
        ui.labs_open = true;
    }),
    ("mac-production", |ui| {
        *ui = mac_fixture();
        ui.settings.meetings.enabled = false;
    }),
    ("mac-activity", |ui| {
        *ui = mac_fixture();
        ui.platform.developer = true;
        ui.activity = activity::ActivityPane::fixture();
        ui.pane = Pane::Activity;
    }),
    // Toggle's cancel shortcut under the shortcut mode, on its default
    // (Escape), and on Wayland, where the desktop binds it.
    ("toggle-cancel", |ui| {
        *ui = mac_fixture();
        ui.settings.shortcut_mode = crate::dictation::ShortcutMode::Toggle;
    }),
    ("portal-toggle-cancel", |ui| {
        *ui = portal_fixture();
        ui.settings.shortcut_mode = crate::dictation::ShortcutMode::Toggle;
    }),
    ("mac-settings-sides", |ui| {
        *ui = mac_fixture();
        ui.advanced = vec![Pane::General];
        ui.settings.dictation_hotkey = HotkeyBinding {
            modifiers: HotkeyModifiers {
                option: Some(crate::app_settings::ModifierSide::Left),
                ..HotkeyModifiers::default()
            },
            key: None,
        };
    }),
    // The first run on macOS: setting up Hex, each permission state, the
    // practice game, and Hex is ready.
    ("mac-onboarding", |ui| {
        mac_first_run(ui, onboarding::MacStep::Model, Page::Download)
    }),
    ("mac-onboarding-permissions", |ui| {
        mac_first_run(ui, onboarding::MacStep::Setup, Page::Permissions)
    }),
    ("mac-onboarding-denied", |ui| {
        mac_first_run(ui, onboarding::MacStep::SetupDenied, Page::Permissions)
    }),
    ("mac-onboarding-stale", |ui| {
        mac_first_run(ui, onboarding::MacStep::SetupStale, Page::Permissions)
    }),
    ("mac-onboarding-restart", |ui| {
        mac_first_run(ui, onboarding::MacStep::SetupRestart, Page::Permissions)
    }),
    ("mac-onboarding-try", |ui| {
        mac_first_run(ui, onboarding::MacStep::TryIt, Page::Try)
    }),
    ("mac-onboarding-reopen", |ui| {
        mac_first_run(ui, onboarding::MacStep::TryItReopen, Page::Try)
    }),
    ("mac-onboarding-ready", |ui| {
        mac_first_run(ui, onboarding::MacStep::Done, Page::Ready)
    }),
    // Its three buttons in a Russian window, the longest of their words.
    ("mac-onboarding-ready-russian", |ui| {
        mac_first_run(ui, onboarding::MacStep::Done, Page::Ready);
        ui.settings.ui_language = Some("ru".into());
        crate::desktop::i18n::apply(Some("ru"));
    }),
];

/// The tour state a Linux first-run scene starts from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FirstRun {
    /// Nothing installed and nothing chosen: nothing downloads.
    Choosing,
    /// The person pressed Download; it is a third of the way.
    Downloading,
    /// The download they started stopped.
    Failed,
    /// The model is installed.
    Ready,
    /// The model is installed, and the desktop asks before Hex's first
    /// paste (GNOME and KDE on Wayland).
    PasteAccess,
}

fn first_run(ui: &mut Ui, state: FirstRun, page: Page) {
    *ui = portal_fixture();
    onboarding::fixture(
        ui,
        crate::desktop::onboarding::Step::Setup,
        state != FirstRun::Choosing,
    );
    match state {
        FirstRun::Choosing => {}
        FirstRun::Downloading => onboarding::set_downloaded(ui, 31),
        FirstRun::Failed => {
            if let Some(view) = &mut ui.view {
                view.transcription.preparing = None;
                view.transcription.error = Some("The connection was reset.".into());
            }
        }
        FirstRun::Ready | FirstRun::PasteAccess => {
            ui.picker.installed = Some(vec![ui.settings.transcription.model]);
            if let Some(view) = &mut ui.view {
                view.transcription.preparing = None;
            }
        }
    }
    if state == FirstRun::PasteAccess {
        ui.platform.paste = Some(paste_access::PasteAccess::Ask);
    }
    onboarding::poll(ui);
    onboarding_kit::live::fixture(ui, page);
}

/// The model step as it opens, with the interface in `language`.
fn first_run_in(ui: &mut Ui, language: &str) {
    first_run(ui, FirstRun::Choosing, Page::Download);
    ui.settings.ui_language = Some(language.into());
    crate::desktop::i18n::apply(Some(language));
    onboarding_kit::live::refresh_choice(ui);
}

fn mac_first_run(ui: &mut Ui, step: onboarding::MacStep, page: Page) {
    *ui = mac_fixture();
    onboarding::mac_fixture(ui, step);
    onboarding_kit::live::fixture(ui, page);
}

/// The Annotate pane with captures `enabled`, a 3-day video retention, and
/// `adjust` applied to what the host shows.
#[cfg(hex_capture)]
fn scrawl_fixture(ui: &mut Ui, enabled: bool, adjust: fn(&mut platform::ScrawlView)) {
    *ui = mac_fixture();
    ui.settings.capture.enabled = enabled;
    ui.settings.capture.media_days = Some(3);
    let mut view = platform::ScrawlView::preview();
    adjust(&mut view);
    ui.platform.scrawl = Some(view);
    ui.pane = Pane::Annotate;
}

/// Labs › Files on a Mac, showing `pane`.
fn files_scene(ui: &mut Ui, pane: files::FilesPane) {
    *ui = mac_fixture();
    ui.settings.labs.files = true;
    ui.files = pane;
    ui.pane = Pane::Files;
}

/// Meetings › Settings with Summaries set to OpenCode, a model saved, and
/// `models` as the OpenCode catalog.
fn summary_model_scene(ui: &mut Ui, models: models::Models) {
    *ui = mac_fixture();
    ui.settings.meetings.summaries = crate::app_settings::MeetingSummaries::OpenCode;
    ui.settings.meetings.opencode_model = Some("anthropic/claude-sonnet-4-5".into());
    ui.models = models;
    ui.meetings.view = meetings::View::Settings;
    ui.pane = Pane::Meetings;
}

fn shortcut_row(ui: &mut Ui, capture: Capture) {
    *ui = mac_fixture();
    ui.settings.dictation_hotkey = HotkeyBinding {
        modifiers: HotkeyModifiers {
            option: Some(ModifierSide::Left),
            ..HotkeyModifiers::default()
        },
        key: None,
    };
    ui.capture = capture;
}

fn listening(message: Option<&'static str>, held: HotkeyModifiers) -> Capture {
    Capture::Listening {
        target: ShortcutTarget::Dictation,
        message,
        held,
    }
}

/// Meeting notes on Codex, in Russian.
/// Files' Original with the player unfolded and playing the second
/// paragraph, or folded, paused on a picked one.
fn files_playback(ui: &mut Ui, playing: bool) {
    files_scene(ui, files::FilesPane::fixture());
    ui.settings.labs.playback = true;
    ui.files.tab = Some(files::Tab::Version(1));
    let of = super::playback::Of::File("preview-interview".into());
    ui.playback = super::playback::Playback::still(of, 25_000, 2_530_000, playing, playing);
}

/// A meeting's transcript with the player unfolded, the third turn
/// playing, or folded, paused on it.
fn meetings_playback(ui: &mut Ui, playing: bool) {
    *ui = mac_fixture();
    meetings_on_codex(ui);
    ui.settings.labs.playback = true;
    ui.meetings.select("preview-notes".into());
    ui.meetings.tab = Some(super::meeting_tabs::MeetingTab::Transcript);
    super::meeting_tabs::requests::fixture(ui, "preview-notes");
    ui.pane = Pane::Meetings;
    let of = super::playback::Of::Meeting("preview-notes".into());
    ui.playback = super::playback::Playback::still(of, 10_500, 1_640_000, playing, playing);
}

fn meetings_on_codex(ui: &mut Ui) {
    ui.settings.labs.ai_cleanup_server = true;
    ui.settings.transcription = crate::transcription::models::TranscriptionSelection {
        model: crate::transcription::models::TranscriptionModelId::WhisperLargeV3Turbo,
        language: "ru".into(),
    };
    ui.files.codex_models = files::FilesPane::fixture_codex_models();
    ui.settings.meetings.notes.writer = Some("codex:gpt-6-luna".into());
}

/// Labs with a few labs on: every store scene's fixture, so a lab reads
/// the same in each.
fn labs_with_some_on(ui: &mut Ui) {
    *ui = mac_fixture();
    for feature in [
        crate::labs::Feature::LiveText,
        crate::labs::Feature::SmartSpacing,
        crate::labs::Feature::TapOrHold,
    ] {
        feature.set(&mut ui.settings, true);
    }
    #[cfg(hex_capture)]
    {
        ui.platform.scrawl = Some(platform::ScrawlView::preview());
    }
    ui.labs_open = true;
}
