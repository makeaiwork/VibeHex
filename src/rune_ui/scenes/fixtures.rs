//! The base windows every scene starts from: a Wayland desktop with the
//! shortcuts portal, its portal-less variant, and a Mac.

use super::super::*;

/// A Mac with meetings on, two call apps, and the Hex Microphone.
pub(in crate::rune_ui) fn mac_fixture() -> Ui {
    let mut ui = fixture(DesktopShortcuts::Registered);
    ui.platform = platform::PlatformState::preview(platform::Capabilities::MACOS);
    ui.host = platform::PlatformHost::Fixture(Vec::new());
    ui.settings.dictation_hotkey = AppSettings::default().dictation_hotkey;
    ui.settings.paste_with_shift = false;
    ui.settings.meetings.enabled = true;
    ui.settings.meetings.summaries = crate::app_settings::MeetingSummaries::Mac;
    ui.settings.meetings.providers = vec![
        crate::app_settings::MeetingProviderChoice {
            key: "com.tinyspeck.slackmacgap".into(),
            label: "Slack huddles".into(),
            policy: crate::app_settings::MeetingPolicy::AlwaysRecord,
        },
        crate::app_settings::MeetingProviderChoice {
            key: "problems.kitlangton.com".into(),
            label: "problems.kitlangton.com".into(),
            policy: crate::app_settings::MeetingPolicy::NeverOffer,
        },
    ];
    ui.settings.experimental.hex_microphone = true;
    ui.microphones = Ok(crate::audio::InputDevices {
        names: vec![
            "Kit Pods".into(),
            "MacBook Pro Microphone".into(),
            "Studio Display Microphone".into(),
        ],
        system_default: Some("Kit Pods".into()),
        default_is_hex_microphone: false,
        ..Default::default()
    });
    ui.meetings = meetings::MeetingsPane::fixture();
    ui
}

pub(in crate::rune_ui) fn portal_fixture() -> Ui {
    fixture(DesktopShortcuts::Portal(vec![
        DesktopGrantedShortcut {
            name: "Dictation".into(),
            trigger: Some("F12".into()),
            unconfirmed: false,
        },
        DesktopGrantedShortcut {
            name: "Paste last dictation".into(),
            trigger: Some("Ctrl+Alt+V".into()),
            unconfirmed: false,
        },
    ]))
}

/// Deterministic settings and service state matching a typical Wayland
/// desktop with the shortcuts portal.
pub(in crate::rune_ui) fn fixture(shortcuts: DesktopShortcuts) -> Ui {
    crate::desktop::i18n::apply(Some("en"));
    let settings = AppSettings {
        dictation_hotkey: HotkeyBinding {
            modifiers: HotkeyModifiers::default(),
            key: Some(HotkeyKey::named("f12")),
        },
        transcription: crate::transcription::models::TranscriptionSelection {
            model: TranscriptionModelId::ParakeetUnifiedEnglish,
            language: "en".into(),
        },
        paste_with_shift: true,
        ..AppSettings::default()
    };
    let mut ui = Ui::with(settings, Service::Fixture(Vec::new()));
    ui.platform.wayland = true;
    ui.models = models::Models::fixture();
    ui.picker.installed = Some(vec![TranscriptionModelId::ParakeetUnifiedEnglish]);
    ui.view = Some(ServiceView {
        listener: DesktopListenerSnapshot {
            running: true,
            warming: false,
            status: "Enabled".into(),
            warning: None,
            notice: None,
            last_failure: None,
            shortcuts,
            shortcuts_configurable: false,
        },
        operation_error: None,
        transcription: DesktopTranscriptionSnapshot {
            downloaded_bytes: 0,
            error: None,
            preparation_stage: None,
            selection: ui.settings.transcription.clone(),
            preparing: None,
        },
        update_status: DesktopUpdateStatus::Current,
        update_notes: None,
        update_blocked: None,
        editing: false,
    });
    ui
}
