//! The onboarding check (`scripts/check-onboarding.sh`, run before every
//! stable release): every onboarding scene against its approved golden in
//! both appearances, and a first run walked through the real onboarding in
//! a clean data folder. Like every golden, these render on Linux only: the
//! macOS and Windows scenes are the same window drawn from fixtures.

use super::super::onboarding_kit::live::Page;
use super::*;
use crate::desktop::onboarding::Step;

/// Every onboarding state a person can see, on macOS and on Linux and
/// Windows. An intentional change refreshes these goldens in the same PR
/// (`HEX_UPDATE_GOLDENS=1`).
pub(super) const SCENES: &[&str] = &[
    // A fresh install: the model step, nothing downloading.
    "onboarding",
    "mac-onboarding",
    // Russian chosen there, and with a file found here that speaks it (the
    // most rows the step shows); the same page in Russian and in German.
    "onboarding-choice-russian",
    "onboarding-choice-russian-found",
    "onboarding-choice-in-russian",
    "onboarding-choice-in-german",
    // Back on the list while the pick downloads, and a pick that failed.
    "onboarding-choice-downloading",
    "onboarding-choice-failed",
    // The download stopped where the practice waits for it; Continue
    // without a model from the start, then Settings.
    "onboarding-download-failed",
    "onboarding-no-model",
    // Models on this computer among the rows; the recommended one among
    // them; ours picked and downloading; Back to the list from the next page.
    "onboarding-model-found",
    "onboarding-model-found-recommended",
    "onboarding-model-found-downloading",
    "onboarding-model-found-back",
    // The demo while the model downloads.
    "onboarding-demo",
    // Permissions (macOS): to do, refused, stale entry, waiting for a
    // restart; Windows' microphone privacy switch.
    "mac-onboarding-permissions",
    "mac-onboarding-denied",
    "mac-onboarding-stale",
    "mac-onboarding-restart",
    "windows-onboarding-microphone",
    // GNOME and KDE on Wayland: the desktop's paste permission, explained.
    "onboarding-paste",
    // Almost ready: the model still downloads.
    "onboarding-waiting",
    // Your turn: the practice game, and the shortcut that didn't start.
    "onboarding-try",
    "mac-onboarding-try",
    "mac-onboarding-reopen",
    // Hex is ready.
    "onboarding-ready",
    "mac-onboarding-ready",
];

/// The pages Skip shows on: the demo, the practice, and the wait for a
/// model still downloading.
const OPTIONAL: &[&str] = &[
    "onboarding-model-found-back",
    "onboarding-demo",
    "onboarding-waiting",
    "onboarding-try",
    "mac-onboarding-try",
    "mac-onboarding-reopen",
];

/// The pages "Continue without a model" shows on: the model step from the
/// start, and where a download failed.
const WITHOUT_MODEL: &[&str] = &[
    "onboarding",
    "mac-onboarding",
    "onboarding-choice-russian",
    "onboarding-choice-russian-found",
    "onboarding-choice-in-russian",
    "onboarding-choice-in-german",
    "onboarding-choice-downloading",
    "onboarding-choice-failed",
    "onboarding-download-failed",
    "onboarding-model-found",
    "onboarding-model-found-recommended",
    "onboarding-model-found-downloading",
];

/// Skip shows only on the pages setup doesn't need: the demo, the practice,
/// and the wait for a model still downloading. It passes over them to the
/// next step setup still needs: permissions, then the model, else Hex is
/// ready. Only on the wait for the model does it close onboarding, with the
/// download going on.
#[test]
fn onboarding_skip() {
    for scene in SCENES {
        let rt = runtime(scene, true, PANE_SIZE);
        assert_eq!(
            rt.find("skip").is_some(),
            OPTIONAL.contains(scene),
            "{scene}: Skip"
        );
    }
    let page = |rt: &Runtime<Ui>| super::super::onboarding_kit::live::page(&rt.state);
    let skip = |rt: &mut Runtime<Ui>, scene: &str, next: Page| {
        tap(rt, "skip");
        settle(rt);
        assert_eq!(page(rt), Some(next), "{scene}: Skip lands here");
        assert!(rt.state.tour.is_some(), "{scene}: onboarding stays open");
        assert_eq!(
            rt.find("skip").is_some(),
            matches!(next, Page::Try | Page::Waiting),
            "{scene}: Skip on {next:?}"
        );
    };

    // macOS, Download pressed, permissions still to grant: the demo skips
    // to them.
    let mut rt = runtime("mac-onboarding", true, PANE_SIZE);
    rt.update(|ui, _| {
        super::super::onboarding_kit::choice::press_primary(ui);
        super::super::onboarding_kit::live::fixture_resume(ui, Page::Demo);
    });
    settle(&mut rt);
    assert_eq!(page(&rt), Some(Page::Demo));
    skip(&mut rt, "mac demo", Page::Permissions);

    // Nothing to grant, the model still downloading: on to Almost ready,
    // and Skip there ends onboarding with the download going on.
    let mut rt = runtime("onboarding-demo", true, PANE_SIZE);
    skip(&mut rt, "onboarding-demo", Page::Waiting);
    let requests = sent(&rt).len();
    tap(&mut rt, "skip");
    settle(&mut rt);
    assert!(rt.state.first_run.is_none(), "Almost ready: Skip closes");
    assert!(rt.state.tour.is_none());
    assert_eq!(sent(&rt).len(), requests, "the download is not cancelled");
    assert!(
        rt.find("transcription-progress").is_some(),
        "Settings shows the download going on"
    );

    // Nothing left to set up: the demo and the practice skip to Hex is ready.
    let mut rt = runtime("onboarding-demo", true, PANE_SIZE);
    rt.update(|ui, tx| {
        ui.picker.installed = Some(vec![ui.settings.transcription.model]);
        if let Some(view) = &mut ui.view {
            view.transcription.preparing = None;
        }
        super::super::onboarding::poll(ui);
        super::super::onboarding_kit::live::poll(ui, tx);
    });
    skip(&mut rt, "onboarding-demo, model ready", Page::Ready);
    for scene in ["onboarding-try", "mac-onboarding-try"] {
        let mut rt = runtime(scene, true, PANE_SIZE);
        skip(&mut rt, scene, Page::Ready);
    }
}

/// Continue without a model: on the model step from the start, and where a
/// download failed or stalled (#894). Nothing downloads (what was
/// downloading stops), the permissions are still asked for, onboarding
/// finishes, and Settings › General then says no model is installed, with
/// Download and Add model file….
#[test]
fn onboarding_continue_without_model() {
    for scene in SCENES {
        let rt = runtime(scene, true, PANE_SIZE);
        assert_eq!(
            rt.find("continue-without-model").is_some(),
            WITHOUT_MODEL.contains(scene),
            "{scene}: Continue without a model"
        );
    }

    // From the start, with nothing to allow: onboarding closes, and nothing
    // was ever asked of the service.
    let mut rt = runtime("onboarding", true, PANE_SIZE);
    tap(&mut rt, "continue-without-model");
    settle(&mut rt);
    assert!(rt.state.first_run.is_none(), "onboarding closed");
    assert!(rt.state.tour.is_none(), "and its tour");
    assert!(sent(&rt).is_empty(), "nothing downloads: {:?}", sent(&rt));
    assert!(
        texts(&rt)
            .contains(&"No speech model installed. Download one or add a model file.".to_owned()),
        "{:?}",
        texts(&rt)
    );
    assert!(rt.find("transcription-download").is_some(), "Download");
    assert!(
        rt.find("transcription-add-file").is_some(),
        "Add model file…"
    );
    assert!(
        texts(&rt).contains(&"VibeHex can't dictate until a speech model is installed".to_owned()),
        "the status says so too"
    );

    // While a pick downloads: its download stops.
    let mut rt = runtime("onboarding-choice-downloading", true, PANE_SIZE);
    tap(&mut rt, "continue-without-model");
    settle(&mut rt);
    assert!(rt.state.first_run.is_none(), "onboarding closed");
    assert!(
        matches!(sent(&rt).last(), Some(Request::CancelModel)),
        "{:?}",
        sent(&rt)
    );

    // macOS, permissions still to grant: they are asked for first, Back
    // returns to the list, and the run ends once they are on.
    let mut rt = runtime("mac-onboarding", true, PANE_SIZE);
    tap(&mut rt, "continue-without-model");
    settle(&mut rt);
    let page = |rt: &Runtime<Ui>| super::super::onboarding_kit::live::page(&rt.state);
    assert_eq!(page(&rt), Some(Page::Permissions));
    assert!(
        rt.state.tour.is_some(),
        "the permissions are still asked for"
    );
    assert!(sent(&rt).is_empty(), "nothing downloads");
    assert!(rt.find("download-pill-shell").is_none(), "no download pill");
    tap(&mut rt, "back");
    assert_eq!(page(&rt), Some(Page::Download));
    assert_eq!(
        rt.state.tour.as_ref().unwrap().flow.choice().decision(),
        crate::desktop::onboarding::Decision::Undecided,
        "back on the list nothing is decided"
    );
    tap(&mut rt, "continue-without-model");
    settle(&mut rt);
    rt.update(|ui, tx| {
        use crate::desktop::onboarding::PermissionState::Ready;
        ui.platform.permissions.microphone = Ready;
        ui.platform.permissions.accessibility = Ready;
        super::super::onboarding::poll(ui);
        super::super::onboarding_kit::live::poll(ui, tx);
        // The granted checks show a beat before the page moves on.
        if let Some(s) = ui.first_run.as_mut() {
            s.now += 1.0;
        }
        super::super::onboarding_kit::live::poll(ui, tx);
    });
    settle(&mut rt);
    assert!(rt.state.first_run.is_none(), "granted: onboarding closed");
    assert!(rt.state.tour.is_none());
    assert!(sent(&rt).is_empty(), "and still nothing downloads");

    // Where the download failed.
    let mut rt = runtime("onboarding-download-failed", true, PANE_SIZE);
    tap(&mut rt, "continue-without-model");
    settle(&mut rt);
    assert!(rt.state.first_run.is_none(), "onboarding closed");
    assert!(
        texts(&rt).contains(&"No speech model installed: The connection was reset.".to_owned()),
        "{:?}",
        texts(&rt)
    );

    // A download that gains nothing for a while offers it too.
    let mut rt = runtime("onboarding-waiting", true, PANE_SIZE);
    assert!(rt.find("continue-without-model").is_none());
    rt.update(|ui, tx| {
        if let Some(s) = ui.first_run.as_mut() {
            s.now += 31.0;
        }
        super::super::onboarding_kit::live::poll(ui, tx);
    });
    settle(&mut rt);
    assert!(rt.find("continue-without-model").is_some(), "stalled");
    tap(&mut rt, "continue-without-model");
    assert!(rt.state.first_run.is_none(), "onboarding closed");
}

#[test]
fn onboarding_goldens() {
    let _goldens = Goldens::collect();
    for scene in SCENES {
        for dark in [true, false] {
            check_golden(scene, dark, PANE_SIZE);
        }
    }
}

/// A first run on Linux or Windows, in the clean data folder
/// `HEX_APPLICATION_SUPPORT_DIR` names (no settings, no models, an empty
/// Hugging Face cache): the real settings load, the real decision to open
/// onboarding, the model step with nothing downloading until Download is
/// pressed, the real download and verification of the model picked there,
/// and every page to Hex is ready. Downloads the recommended model, so only
/// scripts/check-onboarding.sh runs it. The window here is a fixture, not
/// connected to a service: the test does the service's download itself,
/// plays the story on its own clock, and nothing writes the completion
/// marker.
#[test]
#[ignore = "downloads a speech model; run by scripts/check-onboarding.sh"]
fn onboarding_walkthrough() {
    let support = crate::app_paths::support_dir().unwrap();
    assert!(
        std::env::var_os("HEX_APPLICATION_SUPPORT_DIR").is_some(),
        "set HEX_APPLICATION_SUPPORT_DIR to a clean folder"
    );
    assert!(
        std::fs::read_dir(&support).map_or(true, |mut entries| entries.next().is_none()),
        "{} must start empty",
        support.display()
    );
    assert!(
        crate::transcription::found_models::scan().is_empty(),
        "the Hugging Face cache must be empty"
    );

    // A new install: default settings from an empty folder, nothing installed.
    let settings = AppSettings::load().unwrap();
    let mut ui = portal_fixture();
    ui.platform.caps = platform::Capabilities::LINUX;
    ui.settings = settings;
    let installed = |ui: &Ui| {
        let selection = &ui.settings.transcription;
        crate::transcription::models::is_installed(
            crate::transcription::models::definition(selection.model),
            &selection.language,
        )
    };
    assert!(!installed(&ui), "no model is installed yet");
    ui.picker.installed = Some(Vec::new());
    // As the window opens at launch.
    ui.tour = Some(
        super::super::onboarding::Tour::start(&ui.settings, &ui.platform)
            .expect("a new install opens onboarding"),
    );
    super::super::onboarding_kit::live::open(&mut ui);
    assert!(ui.first_run.is_some(), "the first run shows");
    let mut rt = runtime_for(ui, true, PANE_SIZE);
    let step = |rt: &Runtime<Ui>| rt.state.tour.as_ref().map(|tour| tour.flow.step());
    let shown = texts(&rt);
    assert!(
        shown.contains(&"Choose a speech model".to_owned()),
        "{shown:?}"
    );

    // Nothing is asked of the service until Download is pressed; then the
    // recommended model, selected as the step opened, is.
    assert_eq!(step(&rt), Some(Step::Setup));
    settle(&mut rt);
    assert!(sent(&rt).is_empty(), "nothing downloads by itself");
    tap(&mut rt, "continue");
    let (model, language) = match sent(&rt) {
        [Request::ChooseModel { model, language }] => (*model, language.clone()),
        other => panic!("setup should choose a model, sent {other:?}"),
    };
    assert_eq!(model, rt.state.settings.transcription.model);

    // The service's work, for real: download and verify into the clean folder.
    let definition = crate::transcription::models::definition(model);
    let downloaded = std::sync::atomic::AtomicU64::new(0);
    let path = crate::transcription::models::download_with_progress(
        definition,
        &std::sync::atomic::AtomicBool::new(false),
        &downloaded,
    )
    .unwrap_or_else(|error| panic!("downloading {} failed: {error:#}", definition.name));
    assert!(
        path.starts_with(&support),
        "{} is outside the data folder",
        path.display()
    );
    rt.state.settings.transcription = TranscriptionSelection { model, language };
    assert!(installed(&rt.state), "{} is installed", definition.name);
    assert!(
        crate::transcription::models::is_verified(definition),
        "{} is verified against its pin",
        definition.name
    );
    rt.state.picker.installed = Some(vec![model]);

    // The story plays on: the demo, then the practice game once the model
    // is ready, then Hex is ready.
    settle(&mut rt);
    rt.update(|ui, _| super::super::onboarding_kit::live::fixture_resume(ui, Page::Try));
    settle(&mut rt);
    assert_eq!(step(&rt), Some(Step::TryIt));
    assert!(texts(&rt).contains(&"Clear your messages".to_owned()));
    assert!(rt.find("skip").is_some(), "Skip shows on the practice too");
    tap(&mut rt, "done");
    assert!(texts(&rt).contains(&"VibeHex is ready".to_owned()));
    tap(&mut rt, "start-using-hex");
    assert!(
        rt.state.tour.is_none(),
        "Start using VibeHex closes onboarding"
    );
    assert!(rt.state.first_run.is_none());
    println!(
        "onboarding walkthrough: Choose a speech model -> Download -> {} downloaded ({} bytes) -> Your turn -> VibeHex is ready",
        definition.name,
        downloaded.load(std::sync::atomic::Ordering::Relaxed)
    );
}
