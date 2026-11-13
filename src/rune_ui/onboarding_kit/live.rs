//! Hex's first run: the onboarding story, the one onboarding on every OS.
//! A new install (or an upgrade whose setup isn't ready) opens it at
//! launch, and Run setup again and `hex onboarding reset` replay it. The
//! window's tour ([`onboarding`](super::super::onboarding)) is the state
//! machine underneath: it asks for the model the person chose and for the
//! permissions, relaunches when macOS needs it, watches the practice
//! dictation, and records completion. This module shows the story's pages
//! over the window and feeds them that real state each frame.
//!
//! A run with no model starts on the model step ([`choice`](super::choice)):
//! nothing downloads until the person presses Download there. From then the
//! story goes on (the demo, the permissions) while the model downloads
//! under the pill at the top; only the practice waits for it. Back on those
//! pages returns to the list, where the download can be cancelled and
//! another model picked. "Continue without a model" is there from the start.

use crate::desktop::i18n::tr;
use std::sync::OnceLock;
use std::time::Instant;

use rune::{El, TextEdit, Transition, Tx, text};

use super::super::controls::Paint;
use super::super::paste_access::{self, PasteAccess};
use super::super::platform::Os;
use super::super::sidebar::Pane;
use super::super::{Ui, onboarding};
use super::choice::{self, Shown};
use super::fixture::{Language, Speech};
use super::stories::{FRAME, Flow, Onboard, Step, advance_to};
use crate::desktop::onboarding::{
    Decision, Input, ItemStatus, Pick, Practice, Remembered, SetupItem, SetupStatus,
    Step as TourStep, model_choice,
};

/// How long both permission checks show before the page moves on.
const GRANTED_BEAT: f64 = 0.7;
/// How often the first run looks at the model on disk and the permissions.
const CHECK_EVERY: f64 = 0.25;
/// A download that gained no bytes for this long has stalled.
const STALL_AFTER: f64 = 30.0;

/// What the real first run reports to the story's pages.
pub(crate) struct Live {
    /// Hex's own dictation records.
    pub(crate) listening: bool,
    /// The listener runs but its model, microphone, and shortcuts aren't up
    /// yet, so the practice waits instead of inviting a hold.
    pub(crate) warming: bool,
    /// The model download stopped with an error.
    pub(crate) failed: bool,
    /// Why, in one line, such as a proxy or certificate (#894).
    pub(crate) failure: Option<String>,
    /// The download runs but gained nothing for [`STALL_AFTER`].
    pub(crate) stalled: bool,
    /// The download's bytes, and when they last grew.
    progress: Option<(u64, f64)>,
    /// A model file chosen with Use a model file… is being added.
    pub(crate) using_file: bool,
    pub(crate) microphone: ItemStatus,
    pub(crate) accessibility: ItemStatus,
    /// General's "Turn off the mic when not dictating" was on when the
    /// first run opened, so the microphone opens only while you dictate.
    pub(crate) release_microphone: bool,
    /// Every permission is on and one applies only after Hex restarts.
    pub(crate) restart: bool,
    /// macOS asks for Microphone and Accessibility; Windows only shows the
    /// Microphone row while its privacy switch is off.
    pub(crate) accessibility_row: bool,
    /// macOS 27 calls Accessibility "Device Control and Data Access".
    pub(crate) accessibility_renamed: bool,
    /// The shortcut listener failed after setup and Hex won't relaunch by
    /// itself again: Reopen Hex.
    pub(crate) reopen: bool,
    /// Assistant shows in the sidebar (`assistant.enabled`), so Hex is
    /// ready offers Set up with Assistant.
    pub(crate) assistant: bool,
    /// macOS needed a restart and Hex couldn't relaunch itself.
    pub(crate) relaunch_failed: bool,
    /// The desktop's paste permission on GNOME and KDE Wayland: the
    /// permissions page explains it and asks before the practice pastes.
    pub(crate) paste: Option<PasteAccess>,
    /// The model step as it shows, while the tour is open.
    pub(crate) choice: Option<Shown>,
    /// Where the model step stands.
    pub(crate) model_step: ModelStep,
    /// The model step's language menu is open.
    pub(crate) language_open: bool,
    /// What the person started, by name, for the download pill.
    pub(crate) download_name: Option<String>,
    /// The picks already on this computer, as of the last look at the disk.
    here: Vec<Pick>,
    pub(crate) practice: Practice,
    granted_at: Option<f64>,
    checked: Option<Checked>,
    /// Dictation streams send their live words to the practice field.
    _live_words: crate::dictation::live_text::Demand,
}

/// The model step of a run that opened with no model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ModelStep {
    /// A model was here when the run opened: there is nothing to choose.
    #[default]
    None,
    /// The list shows and holds the story until the person decides.
    Choosing,
    /// Decided: a model started, or the run goes on without one. Back
    /// returns to the list.
    Chosen,
}

impl Live {
    /// The story waits on the model step until the person decides.
    pub(crate) fn holds(&self) -> bool {
        self.model_step == ModelStep::Choosing
    }
}

/// The last look at the disk and macOS.
#[derive(Clone, Copy)]
struct Checked {
    at: f64,
    status: SetupStatus,
}

impl Live {
    /// The model can't be had here: its download failed or stalled, so
    /// Continue without a model is offered (#894).
    pub(crate) fn model_blocked(&self) -> bool {
        self.failed || self.stalled
    }

    pub(crate) fn granted_for(&self, now: f64) -> bool {
        self.granted_at.is_some_and(|at| now - at >= GRANTED_BEAT)
    }
}

/// Where a fixture's clock starts.
const FIXTURE_START: f64 = 100.0;

/// Seconds on the first run's own clock. Fixtures keep the story's own
/// time, so a scene shows one settled moment.
fn clock(ui: &Ui) -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    if ui.is_fixture() {
        return ui.first_run.as_ref().map_or(FIXTURE_START, |s| s.now);
    }
    START.get_or_init(Instant::now).elapsed().as_secs_f64() + 1.0
}

/// Every permission is ready, the desktop's paste permission included
/// (or passed over), so the story needs no permissions page.
fn granted(status: SetupStatus, paste: Option<PasteAccess>) -> bool {
    use crate::desktop::onboarding::PermissionState::Ready;
    status.microphone == Ready
        && status.accessibility == Ready
        && paste.is_none_or(PasteAccess::settled)
}

/// Opens the first run over the tour, at the tour's step: at launch when
/// this run needs onboarding, and from Run setup again.
pub(in crate::rune_ui) fn open(ui: &mut Ui) {
    let Some((tour_step, choice)) = ui
        .tour
        .as_ref()
        .map(|tour| (tour.flow.step(), tour.flow.choice().clone()))
    else {
        return;
    };
    if ui.first_run.is_some() {
        return;
    }
    let had_model = ui.picker.selected_installed(&ui.settings);
    let status = ui.platform.setup_status(had_model);
    let granted = granted(status, ui.platform.paste);
    // A run that goes on without a model, with nothing left to allow, is
    // over: the launch after macOS's own restart lands here.
    if !had_model && choice.without() && granted {
        return finish_without_model(ui);
    }
    // Until the person decides, the run starts on the model step, whatever
    // step an earlier launch reached.
    let model_step = match choice.decision() {
        _ if had_model => ModelStep::None,
        Decision::Undecided => ModelStep::Choosing,
        Decision::Started(_) | Decision::Without => ModelStep::Chosen,
    };

    let mut s = Onboard::new(Flow::FirstRun);
    s.speech = Speech::AfterDownload;
    s.wants = if choice.language() == "en" {
        Language::English
    } else {
        Language::Other
    };
    s.caps = ui.settings.dictation_hotkey.keycaps();
    // Without a model there is no download to show either.
    s.had_model = had_model || choice.without();
    s.granted = granted;
    s.home = match ui.platform.caps.os {
        Os::Macos => super::landing::Home::MenuBar,
        Os::Windows => super::landing::Home::Tray,
        Os::Linux => super::landing::Home::Launcher,
    };
    s.live = Some(Live {
        listening: false,
        warming: false,
        failed: false,
        failure: None,
        stalled: false,
        progress: None,
        using_file: false,
        microphone: ItemStatus::Todo,
        accessibility: ItemStatus::Todo,
        release_microphone: ui.settings.release_microphone_while_idle,
        restart: false,
        accessibility_row: ui.platform.caps.permissions,
        accessibility_renamed: ui.platform.accessibility_renamed,
        reopen: false,
        assistant: Pane::Assistant.shown(ui),
        relaunch_failed: false,
        paste: ui.platform.paste,
        choice: None,
        model_step,
        language_open: false,
        download_name: None,
        here: Vec::new(),
        practice: Practice::Waiting,
        granted_at: None,
        checked: None,
        _live_words: crate::dictation::live_text::demand(),
    });
    let now = clock(ui);
    s.now = now;
    match tour_step {
        _ if model_step == ModelStep::Choosing => s.begin_live(now),
        TourStep::Welcome => s.begin_live(now),
        TourStep::Setup => s.jump_to_permissions(),
        TourStep::TryIt if had_model => s.jump_to_try(),
        TourStep::TryIt => s.jump_to_waiting(),
        TourStep::Done => s.jump_to_ready(),
    }
    ui.first_run = Some(s);
    // The tour waits at setup, where it asks for permissions, while the
    // story plays; with everything granted it goes on to the practice.
    if tour_step == TourStep::Welcome {
        onboarding::input(ui, Input::Continue);
    }
}

/// A page of the first run, for scenes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::rune_ui) enum Page {
    /// The model step: `"Choose a speech model"`.
    Download,
    /// The demo, finished: `"Twice as fast"` and Continue.
    Demo,
    Permissions,
    /// `"Almost ready"`: the model still downloads.
    Waiting,
    /// `"Your turn"`: the practice game.
    Try,
    Ready,
}

/// The page the first run shows, for the window tests (Linux only).
#[cfg(all(test, target_os = "linux"))]
pub(in crate::rune_ui) fn page(ui: &Ui) -> Option<Page> {
    Some(match ui.first_run.as_ref()?.step {
        Step::Pick | Step::Started => Page::Download,
        Step::Parked | Step::YourTurn | Step::Hearing | Step::Heard | Step::Done => Page::Demo,
        Step::Permissions => Page::Permissions,
        Step::Waiting => Page::Waiting,
        Step::Try | Step::TryHearing | Step::TryHeard => Page::Try,
        Step::Ready => Page::Ready,
    })
}

/// Scenes: the first run over the tour a fixture walked, played on its own
/// clock to `page` as a person would see it, then held there.
pub(in crate::rune_ui) fn fixture(ui: &mut Ui, page: Page) {
    ui.first_run = None;
    open(ui);
    if let Some(s) = ui.first_run.as_mut() {
        // From the start, as a new install plays it, whatever the tour's step.
        s.now = FIXTURE_START;
        s.step = Step::Pick;
        s.begin_live(FIXTURE_START);
    }
    fixture_resume(ui, page);
}

/// Scenes: plays a fixture's first run on from where it is to `page`.
pub(in crate::rune_ui) fn fixture_resume(ui: &mut Ui, page: Page) {
    // One poll fills in what the tour reports.
    let mut tx = Tx::default();
    poll(ui, &mut tx);
    let Some(s) = ui.first_run.as_mut() else {
        return;
    };
    let reached = |s: &Onboard| match page {
        Page::Download => matches!(s.step, Step::Pick | Step::Started),
        Page::Demo => s.step == Step::Done,
        Page::Permissions => s.step == Step::Permissions,
        Page::Waiting => s.step == Step::Waiting,
        Page::Try => s.step == Step::Try,
        Page::Ready => s.step == Step::Ready,
    };
    if page == Page::Ready {
        s.jump_to_ready();
    }
    for _ in 0..64 {
        if reached(s) {
            break;
        }
        let Some(next) = super::stories::first_run_next(s, s.now) else {
            break;
        };
        s.now += next;
    }
    if page == Page::Try {
        super::game::step(s, s.now);
    }
    // Everything that entered has settled; nothing more is due. The
    // download's progress counts from here, so a scene never shows it
    // stalled.
    s.now += 10.0;
    s.auto_due_at = None;
    if let Some(live) = s.live.as_mut() {
        live.progress = None;
    }
    poll(ui, &mut tx);
}

/// "Run setup again" and `hex onboarding reset`: forgets that onboarding
/// finished and opens it now. Settings, history, models, and permissions
/// stay as they are, so the run finds them ready and passes their steps.
pub(crate) fn restart(ui: &mut Ui, tx: &mut Tx) -> Result<(), String> {
    if ui.live() {
        crate::desktop::onboarding::reset().map_err(|error| format!("{error:#}"))?;
    }
    ui.select_pane(super::super::Pane::General, tx);
    ui.first_run = None;
    ui.tour = Some(onboarding::Tour::replay(&ui.settings, &ui.platform));
    open(ui);
    tx.settle();
    if ui.first_run.is_none() {
        return Err("VibeHex could not open onboarding".into());
    }
    Ok(())
}

/// The model step's rows and what is on this computer, as of now: a press
/// on the step shows at once, without waiting for the next look at the disk.
pub(in crate::rune_ui) fn refresh_choice(ui: &mut Ui) {
    let Some(view) = ui.tour.as_ref().map(|tour| tour.flow.view()) else {
        return;
    };
    let here = choice::here(ui, &view.choice);
    let shown = choice::shown(ui, &view, &here);
    if let Some(live) = ui.first_run.as_mut().and_then(|s| s.live.as_mut()) {
        live.here = here;
        live.choice = Some(shown);
    }
}

/// Feeds the first run fresh state and moves it on; true while it shows.
pub(in crate::rune_ui) fn poll(ui: &mut Ui, tx: &mut Tx) -> bool {
    if ui.first_run.is_none() {
        return false;
    }
    let now = clock(ui);
    let view = ui.tour.as_ref().map(|tour| tour.flow.view());
    let tour_model = view.as_ref().map_or(ItemStatus::Todo, |view| view.model);
    let decision = view
        .as_ref()
        .map_or(Decision::Undecided, |view| view.choice.decision());
    let started = matches!(decision, Decision::Started(_));
    // What the person started: its name on the pill, its bytes in it.
    let model = view
        .as_ref()
        .and_then(|view| choice::started_model(ui, &view.choice));
    let download_name = view
        .as_ref()
        .and_then(|view| choice::started_name(ui, &view.choice));
    let caps = ui.settings.dictation_hotkey.keycaps();
    let warming = ui.view.as_ref().is_some_and(|view| view.listener.warming);
    let (preparing, downloaded_bytes) = ui.view.as_ref().map_or((false, 0), |view| {
        let t = &view.transcription;
        (
            started && model.is_some_and(|m| t.preparing == Some(m.id)),
            t.downloaded_bytes,
        )
    });
    let failure = super::super::model_import::download_failure(ui);
    let using_file = super::super::model_import::using_file(ui);
    // The disk and macOS checks run a few times a second, not every frame.
    let checked = ui
        .first_run
        .as_ref()
        .and_then(|s| s.live.as_ref())
        .and_then(|live| live.checked)
        // A fixture's clock stands still, so it looks every time.
        .filter(|checked| !ui.is_fixture() && now - checked.at < CHECK_EVERY);
    let (checked, here) = match checked {
        Some(checked) => (checked, None),
        None => {
            let installed = ui.picker.selected_installed(&ui.settings);
            (
                Checked {
                    at: now,
                    status: ui.platform.setup_status(installed),
                },
                view.as_ref().map(|view| choice::here(ui, &view.choice)),
            )
        }
    };
    let here = here.or_else(|| {
        ui.first_run
            .as_ref()
            .and_then(|s| s.live.as_ref())
            .map(|live| live.here.clone())
    });
    let shown = view
        .as_ref()
        .map(|view| choice::shown(ui, view, here.as_deref().unwrap_or_default()));
    let ready = tour_model == ItemStatus::Ready;
    let status = checked.status;
    // Setup that is already ready never moves the tour past it by itself,
    // and the tour watches the practice dictation only once it has.
    if status.ready()
        && ui
            .tour
            .as_ref()
            .is_some_and(|tour| tour.flow.step() == TourStep::Setup)
    {
        onboarding::input(ui, Input::Continue);
    }
    let asks_permissions = ui.platform.caps.permissions;
    let accessibility_renamed = ui.platform.accessibility_renamed;
    let relaunch_failed = ui.platform.relaunch_failed;
    let paste = ui.platform.paste;
    let live_window = !ui.is_fixture();
    let assistant = Pane::Assistant.shown(ui);
    let Some(s) = ui.first_run.as_mut() else {
        return false;
    };
    let Some(live) = s.live.as_mut() else {
        return false;
    };
    live.warming = warming;
    live.assistant = assistant;
    live.checked = Some(checked);
    // Linux and Windows ask for no permissions: both rows are ready.
    if !asks_permissions {
        live.microphone = ItemStatus::Ready;
        live.accessibility = ItemStatus::Ready;
    }

    live.paste = paste;

    // Permissions, as the tour's setup rows show them.
    if let Some(view) = &view {
        for row in &view.rows {
            match row.item {
                SetupItem::Microphone => live.microphone = row.status,
                SetupItem::Accessibility => live.accessibility = row.status,
                _ => {}
            }
        }
        live.restart = view.restart;
        live.accessibility_renamed = accessibility_renamed;
        live.relaunch_failed = relaunch_failed;
        live.reopen = view.reopen;
        live.practice = view.practice;
    }

    live.choice = shown;
    live.download_name = download_name;
    if let Some(here) = here {
        live.here = here;
    }
    let granted = granted(status, paste);
    if granted {
        live.granted_at.get_or_insert(now);
    } else {
        live.granted_at = None;
    }
    live.listening = live.practice == Practice::Listening;
    // A run without a model is over once nothing is left to allow.
    if decision == Decision::Without && live.granted_for(now) {
        finish_without_model(ui);
        return false;
    }

    // The download the person started; until they start one there is none
    // to fail or stall.
    live.failed =
        started && !ready && !preparing && (tour_model == ItemStatus::Failed || failure.is_some());
    live.progress = (preparing && !ready).then_some(match live.progress {
        Some((bytes, since)) if bytes == downloaded_bytes => (bytes, since),
        _ => (downloaded_bytes, now),
    });
    live.stalled = live
        .progress
        .is_some_and(|(_, since)| now - since >= STALL_AFTER);
    live.failure = failure;
    live.using_file = using_file;
    s.granted = granted;
    s.caps = caps;
    if let Some(download) = s.download.as_mut() {
        if let Some(total) = model.and_then(|m| m.download_bytes()) {
            download.total = total;
        }
        if ready {
            download.downloaded = download.total;
        } else if preparing {
            download.downloaded = downloaded_bytes.min(download.total.saturating_sub(1));
        } else if !started {
            download.downloaded = 0;
        }
    }

    // Hex's state arrives between frames, so the first run keeps polling;
    // a fixture shows one moment and stays still.
    advance_to(s, now, tx);
    if live_window {
        tx.wake_after(FRAME);
    }
    true
}

/// The first run over the whole window, while it shows, and the model
/// step's open language menu above it.
pub(in crate::rune_ui) fn overlay(ui: &Ui, p: Paint, reduced: bool, layer: &mut Vec<El>) {
    let Some(s) = ui.first_run.as_ref() else {
        return;
    };
    s.reduced.set(reduced);
    layer.push(
        super::flows::view(p, s)
            .key("first-run")
            .role("dialog")
            .label(tr("Setting up VibeHex"))
            .absolute(0.0, 0.0)
            .on_press(|_: &mut Ui| {}),
    );
    if matches!(s.step, Step::Pick | Step::Started) {
        choice::language_menu(p, s, layer);
    }
    // "All models…" opens the picker above the model step.
    layer.extend(
        super::super::picker::over_first_run(ui, p)
            .into_iter()
            .flatten(),
    );
}

/// A story handler run on the window's first run.
pub(super) fn on_ui(f: fn(&mut Onboard)) -> impl Fn(&mut Ui) + 'static {
    move |ui: &mut Ui| {
        if let Some(s) = ui.first_run.as_mut() {
            f(s);
        }
    }
}

pub(super) fn press_continue(ui: &mut Ui) {
    if let Some(s) = ui.first_run.as_mut() {
        s.move_on();
    }
}

/// Skip on the demo or the practice: passes over that part to the next
/// step setup still needs. It never records completion; only Hex is ready
/// (or Continue without a model) does. On `"Almost ready"` it passes over
/// the practice that waits for the model: the run ends, and the model goes
/// on downloading behind Settings.
pub(in crate::rune_ui) fn skip(ui: &mut Ui) {
    if ui
        .first_run
        .as_ref()
        .is_some_and(|s| s.step == Step::Waiting)
    {
        return finish(ui);
    }
    if let Some(s) = ui.first_run.as_mut() {
        s.skip_optional();
    }
}

/// "Continue without a model": on the model step from the start, and where
/// a download failed or stalled (#894). Whatever was downloading stops, the
/// dictation language chosen here is saved, the permissions are still asked
/// for, and onboarding finishes with no model; Settings says so with
/// Download and Add model file….
pub(in crate::rune_ui) fn continue_without_model(ui: &mut Ui) {
    let Some(s) = ui.first_run.as_ref() else {
        return;
    };
    let offered = s.live.as_ref().is_some_and(|live| {
        live.model_blocked()
            || (live.model_step != ModelStep::None && matches!(s.step, Step::Pick | Step::Started))
    });
    if !offered {
        return;
    }
    let granted = s.granted;
    // The language chosen here is kept for Settings and the picker.
    onboarding::keep_language(ui);
    onboarding::input(ui, Input::WithoutModel);
    if granted {
        return finish_without_model(ui);
    }
    // macOS may restart Hex for a permission: the next launch goes on
    // without a model too.
    if ui.live() {
        model_choice::remember(&Remembered::Without);
    }
    if let Some(s) = ui.first_run.as_mut() {
        s.had_model = true;
        if let Some(live) = s.live.as_mut() {
            live.model_step = ModelStep::Chosen;
            live.language_open = false;
        }
        s.jump_to_permissions();
    }
}

/// Onboarding is over with no model, by the person's choice.
fn finish_without_model(ui: &mut Ui) {
    if ui.live() {
        model_choice::forget();
        if !crate::desktop::onboarding::completion_recorded()
            && let Err(error) = crate::desktop::onboarding::record_completion()
        {
            tracing::warn!(%error, "could not record onboarding completion");
        }
    }
    ui.first_run = None;
    ui.tour = None;
}

/// Whether finishing now counts setup as finished: the permissions ready
/// and the model here or on its way.
pub(in crate::rune_ui) fn completes(ui: &Ui) -> bool {
    let installed = ui.picker.selected_installed(&ui.settings);
    let on_its_way = ui.tour.as_ref().is_some_and(|tour| {
        tour.flow.choice().started().is_some() && tour.flow.view().model != ItemStatus::Failed
    });
    ui.platform.setup_status(installed || on_its_way).ready()
}

/// "Start using Hex" (and Skip on `"Almost ready"`): closes the first run
/// and the tour under it. Setup counts as finished as [`completes`] says;
/// a download still running goes on, and a launch that interrupts it
/// resumes it.
pub(super) fn finish(ui: &mut Ui) {
    if ui.live()
        && completes(ui)
        && !crate::desktop::onboarding::completion_recorded()
        && let Err(error) = crate::desktop::onboarding::record_completion()
    {
        tracing::warn!(%error, "could not record onboarding completion");
    }
    ui.first_run = None;
    ui.tour = None;
}

/// "Set up with Assistant", beside Start using VibeHex: everything Start
/// does, then the window on Assistant in a new conversation with guided
/// setup begun, round one showing, as "Set VibeHex up for me" begins it.
pub(in crate::rune_ui) fn set_up_with_assistant(ui: &mut Ui, tx: &mut Tx) {
    finish(ui);
    super::super::assistant::set_up(ui, tx);
}

/// The person decided on the model step: the story goes on, with the
/// download pill unless the model was already `here`.
pub(super) fn model_chosen(ui: &mut Ui, here: bool) {
    if let Some(s) = ui.first_run.as_mut() {
        s.had_model = here;
        if let Some(live) = s.live.as_mut() {
            live.model_step = ModelStep::Chosen;
            live.language_open = false;
            live.failed = false;
            live.stalled = false;
            live.progress = None;
        }
        s.move_on();
    }
}

/// Back to the model step, to change the pick. What was started goes on
/// until it is cancelled or replaced there; a run without a model is
/// undecided again.
pub(in crate::rune_ui) fn back_to_models(ui: &mut Ui) {
    let without = ui
        .tour
        .as_ref()
        .is_some_and(|tour| tour.flow.choice().without());
    if without {
        onboarding::input(ui, Input::CancelModel);
        if ui.live() {
            model_choice::forget();
        }
    }
    if let Some(s) = ui.first_run.as_mut() {
        s.back_to_download();
        if let Some(live) = s.live.as_mut() {
            live.model_step = ModelStep::Choosing;
        }
    }
    refresh_choice(ui);
}

/// "Reopen Hex" once the listener failed and Hex won't relaunch itself.
pub(super) fn reopen(ui: &mut Ui) {
    onboarding::input(ui, Input::Reopen);
}

/// "Use a model file…" when the download is blocked: a GGUF chosen here is
/// imported and selected, and the first run moves on once it is ready.
pub(super) fn use_file(ui: &mut Ui) {
    super::super::model_import::use_file(ui);
}

/// "Try again" where the download failed: the person's pick starts again.
pub(super) fn retry_download(ui: &mut Ui) {
    let Some(live) = ui.first_run.as_mut().and_then(|s| s.live.as_mut()) else {
        return;
    };
    live.failed = false;
    live.stalled = false;
    live.progress = None;
    onboarding::input(ui, Input::Grant(SetupItem::Model));
}

/// Allow on a permission row.
pub(super) fn allow(item: SetupItem) -> impl Fn(&mut Ui) + 'static {
    move |ui: &mut Ui| onboarding::input(ui, Input::Grant(item))
}

/// Allow on the paste permission: the desktop's dialog opens.
pub(super) fn allow_paste(ui: &mut Ui) {
    paste_access::ask(ui);
}

/// Continue after the desktop's paste permission was refused: the desktop
/// asks again at the first paste.
pub(super) fn paste_later(ui: &mut Ui) {
    paste_access::later(ui);
    if let Some(s) = ui.first_run.as_mut() {
        s.move_on();
    }
}

/// "Restart Hex" once macOS applies a grant only after a restart.
pub(super) fn restart_hex(ui: &mut Ui) {
    onboarding::input(ui, Input::Restart);
}

pub(super) fn practice(ui: &mut Ui) -> &mut TextEdit {
    match ui.first_run.as_mut() {
        Some(s) => &mut s.practice,
        // The field shows only with the first run.
        None => Box::leak(Box::default()),
    }
}

pub(super) fn practice_changed(ui: &mut Ui, tx: &mut Tx) {
    if let Some(s) = ui.first_run.as_mut() {
        s.on_practice_change(tx);
    }
}

/// What the practice says in place of the shortcut invitation while the
/// listener loads its model, microphone, and shortcuts.
pub(super) fn warming_copy() -> &'static str {
    tr("Getting VibeHex ready to listen…")
}

/// What the practice dictation's last try needs said.
pub(super) fn hint_copy(live: &Live) -> Option<&'static str> {
    Some(match live.practice {
        Practice::TooShort => tr("Hold the shortcut a little longer while you speak."),
        Practice::NothingHeard => tr("VibeHex didn't hear any words. Try again a little louder."),
        Practice::Failed => tr("That didn't work. Try again."),
        Practice::Transcribing => tr("Typing what you said…"),
        _ => return None,
    })
}

/// [`hint_copy`] under the practice field.
pub(super) fn hint(p: Paint, live: &Live) -> Option<El> {
    let copy = hint_copy(live)?;
    Some(
        text(copy)
            .key("practice-hint")
            .font(p.k.ty.small)
            .color(p.muted())
            .transition(Transition::opacity()),
    )
}
