//! The first run's state machine in the window: [`Tour`] feeds
//! `onboarding::Flow` the permissions and the model's progress, asks the
//! service for the model the person started (never one by itself), asks for
//! permissions, relaunches when macOS needs it, watches the practice
//! dictation, and records completion. The first run the person sees is the
//! onboarding story over it ([`onboarding_kit::live`]), the one onboarding
//! on every OS.
//!
//! [`onboarding_kit::live`]: super::onboarding_kit::live

use std::time::{Duration, Instant};

use super::Request;
use super::Ui;
use super::platform::{PlatformRequest, PlatformState};
use crate::app_settings::AppSettings;
use crate::desktop::onboarding::{
    Decision, Effect, Flow, Input, ModelProgress, PermissionState, Pick, Remembered, SetupItem,
    Step, model_choice,
};
use crate::transcription::models::{ModelDefinition, TranscriptionModelId, TranscriptionSelection};

/// How long the ready check shows before setup moves on.
const ADVANCE_BEAT: Duration = Duration::from_millis(700);

pub(super) struct Tour {
    pub(super) flow: Flow,
    advance_at: Option<Instant>,
    completed: bool,
    /// The model last asked of the service for the person's pick.
    asked: Option<TranscriptionModelId>,
    /// The saved selection when the person started their pick: a model
    /// installed before it is not the one they wait for.
    started_from: Option<TranscriptionSelection>,
    /// This run opened with no model, so the model is the person's to
    /// choose: until they do, none counts as theirs, not even the
    /// preselected one turning up on this computer.
    chooses: bool,
}

impl Tour {
    fn new(flow: Flow, completed: bool, chooses: bool) -> Self {
        Self {
            flow,
            advance_at: None,
            completed,
            asked: None,
            started_from: None,
            chooses,
        }
    }

    /// The tour from Welcome, as on a new install: Run setup again.
    pub(super) fn replay(settings: &AppSettings, platform: &PlatformState) -> Tour {
        let installed = super::picker::Picker::default().selected_installed(settings);
        Tour::new(
            Flow::new(
                Step::Welcome,
                platform.setup_status(installed),
                platform.caps.permissions,
                platform.caps.practice_dictation,
            )
            .preselect(&settings.transcription),
            false,
            !installed,
        )
    }

    /// The tour when this run still needs it.
    pub(super) fn start(settings: &AppSettings, platform: &PlatformState) -> Option<Tour> {
        let installed = super::picker::Picker::default().selected_installed(settings);
        let status = platform.setup_status(installed);
        let completed = crate::desktop::onboarding::completion_recorded();
        let resume = crate::desktop::onboarding::take_resume();
        let saved = (!completed)
            .then(crate::desktop::onboarding::saved_step)
            .flatten();
        if installed && !cfg!(test) {
            model_choice::forget();
        }
        (resume.is_some() || (!completed && (!status.ready() || saved.is_some()))).then(|| {
            let flow = Flow::new(
                resume.or(saved).unwrap_or(Step::Welcome),
                status,
                platform.caps.permissions,
                platform.caps.practice_dictation,
            )
            .preselect(&settings.transcription);
            // What the person decided before this launch: their download
            // goes on, and a run without a model stays without one. With a
            // model installed there is nothing left to resume.
            let remembered = (!installed).then(model_choice::remembered).flatten();
            Tour::new(
                match remembered {
                    Some(remembered) => flow.resume(remembered),
                    None => flow,
                },
                completed,
                !installed,
            )
        })
    }
}

/// The model General and the tour wait for while none is installed: the one
/// the service prepares, whichever the person asked for, else the saved one.
pub(super) fn awaited_model(ui: &Ui) -> Option<&'static ModelDefinition> {
    let selected = crate::transcription::models::validate(&ui.settings.transcription).ok();
    let preparing = ui
        .view
        .as_ref()
        .and_then(|view| view.transcription.preparing)
        .filter(|_| !ui.picker.selected_installed(&ui.settings))
        .map(crate::transcription::models::definition);
    preparing.or(selected)
}

pub(super) fn progress(ui: &Ui) -> (bool, ModelProgress) {
    let installed = ui.picker.selected_installed(&ui.settings);
    let transcription = ui.view.as_ref().map(|view| &view.transcription);
    let downloading = awaited_model(ui).and_then(|model| downloading(ui, model));
    (
        installed,
        ModelProgress {
            installed,
            downloading,
            failed: downloading.is_none() && transcription.is_some_and(|t| t.error.is_some()),
        },
    )
}

/// How far the service has downloaded `model`, while it prepares it.
fn downloading(ui: &Ui, model: &ModelDefinition) -> Option<f32> {
    let transcription = &ui.view.as_ref()?.transcription;
    (transcription.preparing == Some(model.id)).then(|| {
        model.download_bytes().map_or(0.0, |bytes| {
            (transcription.downloaded_bytes as f32 / bytes as f32).clamp(0.0, 1.0)
        })
    })
}

/// Whether the model the tour waits for is installed: the saved one; in a
/// run that opened with no model, none until the person decides; and once
/// they started a pick, only what was saved for it (or since).
fn reached(ui: &Ui, installed: bool) -> bool {
    let Some(tour) = &ui.tour else {
        return installed;
    };
    let choice = tour.flow.choice();
    if tour.chooses && choice.decision() == Decision::Undecided {
        return false;
    }
    installed
        && (choice.started().is_none()
            || tour.started_from.as_ref() != Some(&ui.settings.transcription)
            || tour.asked == Some(ui.settings.transcription.model))
}

/// Feeds the flow fresh permission and model state; true when the tour
/// changed.
pub(super) fn poll(ui: &mut Ui) -> bool {
    resume_after_setup(ui);
    // Onboarding looks for models already on this computer once; they join
    // the model step's rows when the scan answers.
    if ui.tour.is_some() {
        super::found::scan_for_setup(ui);
    }
    let (installed, mut model) = progress(ui);
    let installed = reached(ui, installed);
    model.installed = installed;
    // The pick's own download, whichever model is saved.
    if let Some(asked) = ui.tour.as_ref().and_then(|tour| tour.asked) {
        model.downloading = downloading(ui, crate::transcription::models::definition(asked));
        model.failed &= model.downloading.is_none();
    }
    // A found file that failed its check failed the pick.
    model.failed |= ui.found.error.is_some() && !ui.found.adopting();
    let status = ui.platform.setup_status(installed);
    let accessibility_stale = ui
        .platform
        .stale(crate::desktop::onboarding::PermissionKind::Accessibility);
    let live = ui.live();
    let Some(tour) = &mut ui.tour else {
        return false;
    };
    let before = tour.flow.view();
    let step = before.step;
    let mut effect = tour.flow.on(Input::Status(status, model));
    tour.flow.set_accessibility_stale(accessibility_stale);
    if tour
        .advance_at
        .is_some_and(|at| at.elapsed() >= ADVANCE_BEAT)
    {
        tour.advance_at = None;
        if tour.flow.step() == Step::Setup {
            effect = tour.flow.on(Input::Continue);
        }
    }
    if status.ready() && !tour.completed {
        tour.completed = true;
        if live && let Err(error) = crate::desktop::onboarding::record_completion() {
            tracing::warn!(%error, "could not record onboarding completion");
        }
    }
    // The model the person asked for is here: nothing is left to resume.
    if live && installed && tour.flow.choice().started().is_some() {
        model_choice::forget();
    }
    let practicing = tour.flow.step() == Step::TryIt;
    apply(ui, effect);
    let heard = ui.host.tour_inputs(practicing);
    let changed = !heard.is_empty();
    for heard in heard {
        input(ui, heard);
    }
    remember_step(ui, step);
    let after = ui.tour.as_ref().map(|tour| tour.flow.view());
    changed || after.as_ref() != Some(&before)
}

pub(super) fn input(ui: &mut Ui, input: Input) {
    let Some(tour) = &mut ui.tour else {
        return;
    };
    let step = tour.flow.step();
    let effect = tour.flow.on(input);
    apply(ui, effect);
    remember_step(ui, step);
}

/// Saves a new step so a run quit part-way resumes there. The story sounds
/// the success chime itself, on Hex is ready.
fn remember_step(ui: &Ui, before: Step) {
    if let Some(tour) = &ui.tour
        && tour.flow.step() != before
        && ui.live()
    {
        crate::desktop::onboarding::record_step(tour.flow.step());
    }
}

fn apply(ui: &mut Ui, effect: Effect) {
    match effect {
        Effect::DownloadModel => start_model(ui),
        Effect::CancelModel => {
            if ui.live() {
                model_choice::forget();
            }
            ui.found.error = None;
            if let Some(tour) = &mut ui.tour {
                tour.asked = None;
                tour.started_from = None;
            }
            ui.send(Request::CancelModel);
        }
        Effect::AdvanceSoon => {
            if let Some(tour) = &mut ui.tour {
                tour.advance_at = Some(Instant::now());
            }
        }
        Effect::RequestMicrophone => ui.ask(PlatformRequest::Permission(
            crate::desktop::onboarding::PermissionAction::RequestMicrophone,
        )),
        Effect::ShowGuide(kind) => ui.ask(PlatformRequest::Permission(
            crate::desktop::onboarding::PermissionAction::ShowGuide(kind),
        )),
        Effect::ResetPermission(kind) => ui.ask(PlatformRequest::Permission(
            crate::desktop::onboarding::PermissionAction::ResetAndPrompt(kind),
        )),
        Effect::Relaunch => ui.ask(PlatformRequest::Relaunch),
        Effect::Finish => {
            ui.tour = None;
            ui.first_run = None;
        }
        Effect::None => {}
    }
}

/// Asks the service for the row the person started: a catalog model with
/// the chosen language, or a found file once its check registers it.
fn start_model(ui: &mut Ui) {
    let Some(choice) = ui.tour.as_ref().map(|tour| tour.flow.choice().clone()) else {
        return;
    };
    ui.found.error = None;
    let saved = ui.settings.transcription.clone();
    if let Some(tour) = &mut ui.tour {
        tour.asked = None;
        tour.started_from = Some(saved);
    }
    let model = match choice.started() {
        Some(Pick::Model(model)) => model,
        Some(Pick::Found(index)) => match ui.found.models.get(index).cloned() {
            Some(found) => match found.catalog {
                Some(model) => model,
                // Hashed once and registered in place first, then prepared.
                None => return super::found::adopt(ui, found, true),
            },
            None => return,
        },
        None => return,
    };
    prepare(ui, choice.selection_for(model));
}

/// Asks the service for `selection`, the person's choice, and remembers it
/// so a launch that interrupts the download goes on with it.
pub(super) fn prepare(ui: &mut Ui, selection: TranscriptionSelection) {
    if crate::transcription::models::validate(&selection).is_err() {
        return;
    }
    if ui.live() {
        model_choice::remember(&Remembered::Model(selection.clone()));
    }
    if let Some(tour) = &mut ui.tour {
        tour.asked = Some(selection.model);
    }
    ui.send(Request::ChooseModel {
        model: selection.model,
        language: selection.language,
    });
}

/// A model chosen for the open tour from outside its list (the full picker,
/// a model file): it is the person's pick, started as Download starts one.
pub(super) fn choose(ui: &mut Ui, model: TranscriptionModelId) {
    input(ui, Input::Pick(Pick::Model(model)));
    input(ui, Input::Grant(SetupItem::Model));
}

/// Once per launch, with setup already finished: the download the person
/// started in onboarding and a quit interrupted goes on. A launch with a
/// model installed, or one after "Continue without a model", starts nothing.
fn resume_after_setup(ui: &mut Ui) {
    static CHECKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !ui.live() || CHECKED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    // An open tour resumes it itself (`Flow::resume`).
    if ui.tour.is_some() {
        return;
    }
    match model_choice::remembered() {
        Some(Remembered::Model(selection)) if !ui.picker.selected_installed(&ui.settings) => {
            prepare(ui, selection);
        }
        Some(Remembered::Model(_)) => model_choice::forget(),
        _ => {}
    }
}

/// A first run at `step`, with the selected model not yet installed; with
/// `downloading`, the person pressed Download on the preselected row.
pub(super) fn fixture(ui: &mut Ui, step: Step, downloading: bool) {
    ui.picker.installed = Some(Vec::new());
    let status = ui.platform.setup_status(false);
    let mut flow = Flow::new(
        Step::Welcome,
        status,
        ui.platform.caps.permissions,
        ui.platform.caps.practice_dictation,
    )
    .preselect(&ui.settings.transcription);
    if step != Step::Welcome {
        flow.on(Input::Continue);
    }
    ui.tour = Some(Tour::new(flow, false, true));
    if downloading {
        input(ui, Input::Grant(SetupItem::Model));
        set_downloaded(ui, 62);
        poll(ui);
    }
}

pub(super) fn set_downloaded(ui: &mut Ui, percent: u64) {
    // The model the person started, else the preselected one.
    let started = ui
        .tour
        .as_ref()
        .and_then(|tour| tour.flow.choice().started());
    let model = match started {
        Some(Pick::Model(model)) => model,
        _ => ui.settings.transcription.model,
    };
    if let Some(view) = &mut ui.view {
        let bytes = crate::transcription::models::definition(model)
            .download_bytes()
            .unwrap_or(0);
        view.transcription.preparing = Some(model);
        view.transcription.downloaded_bytes = bytes * percent / 100;
    }
}

/// The macOS tour's states, walked as a person would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MacStep {
    /// The model step as it opens: nothing chosen, nothing downloading.
    Model,
    Setup,
    SetupStale,
    SetupDenied,
    /// Every permission on, Accessibility waiting for a restart.
    SetupRestart,
    TryIt,
    /// The shortcut listener failed after setup.
    TryItReopen,
    Done,
}

/// Walks a fresh macOS first run to `state`. `ui.platform` must carry
/// macOS capabilities.
pub(super) fn mac_fixture(ui: &mut Ui, state: MacStep) {
    use PermissionState::{NeedsRequest, NeedsRestart, NeedsSettings, Ready};
    let fresh = crate::desktop::onboarding::SetupStatus {
        microphone: NeedsRequest,
        accessibility: NeedsSettings,
        transcription_model: false,
    };
    let installed = matches!(state, MacStep::TryIt | MacStep::TryItReopen | MacStep::Done);
    let permissions = match state {
        MacStep::Model | MacStep::Setup => fresh,
        MacStep::SetupStale => crate::desktop::onboarding::SetupStatus {
            microphone: Ready,
            ..fresh
        },
        MacStep::SetupDenied => crate::desktop::onboarding::SetupStatus {
            microphone: NeedsSettings,
            ..fresh
        },
        MacStep::SetupRestart => crate::desktop::onboarding::SetupStatus {
            microphone: Ready,
            accessibility: NeedsRestart,
            ..fresh
        },
        _ => crate::desktop::onboarding::SetupStatus {
            microphone: Ready,
            accessibility: Ready,
            transcription_model: true,
        },
    };
    ui.platform.permissions = fresh;
    ui.picker.installed = Some(Vec::new());
    ui.tour = Some(Tour::new(
        Flow::new(Step::Welcome, fresh, true, true).preselect(&ui.settings.transcription),
        false,
        true,
    ));
    poll(ui);
    if state != MacStep::Model {
        // The person pressed Download on the preselected row.
        input(ui, Input::Grant(SetupItem::Model));
    }
    input(ui, Input::Continue);
    if state == MacStep::SetupStale {
        for item in [SetupItem::Microphone, SetupItem::Accessibility] {
            input(ui, Input::Grant(item));
        }
        ui.platform
            .note_stale(crate::desktop::onboarding::PermissionKind::Accessibility);
    }
    ui.platform.permissions = permissions;
    if installed {
        ui.picker.installed = Some(vec![ui.settings.transcription.model]);
        if let Some(view) = &mut ui.view {
            view.transcription.preparing = None;
        }
    } else if state != MacStep::Model {
        set_downloaded(ui, 31);
    }
    poll(ui);
    if installed {
        input(ui, Input::Continue);
    }
    if let Some(tour) = &mut ui.tour {
        tour.advance_at = None;
    }
    if state == MacStep::TryItReopen {
        input(
            ui,
            Input::ListenerFailed {
                relaunch_allowed: false,
            },
        );
    }
    if state == MacStep::Done {
        input(ui, Input::Skip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_stale_fixture_marks_accessibility_stale_and_dispatches_reset() {
        let mut ui = crate::rune_ui::mac_fixture();
        mac_fixture(&mut ui, MacStep::SetupStale);
        let rows = ui.tour.as_ref().unwrap().flow.view().rows;
        assert!(rows.iter().any(|row| row.item == SetupItem::Accessibility
            && row.status == crate::desktop::onboarding::ItemStatus::Stale));
        input(&mut ui, Input::Grant(SetupItem::Accessibility));
        assert!(
            !ui.platform
                .stale(crate::desktop::onboarding::PermissionKind::Accessibility)
        );
        match &ui.host {
            crate::rune_ui::platform::PlatformHost::Fixture(sent) => {
                assert_eq!(
                    sent.last(),
                    Some(&PlatformRequest::Permission(
                        crate::desktop::onboarding::PermissionAction::ResetAndPrompt(
                            crate::desktop::onboarding::PermissionKind::Accessibility
                        )
                    ))
                );
            }
            _ => panic!("expected fixture host"),
        }
    }
}
