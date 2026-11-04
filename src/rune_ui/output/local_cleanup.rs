//! Labs › Clean up on your own server › Built in, in Output: the models Hex
//! downloads for cleanup, each with Download (and its progress and Cancel),
//! Use, and Delete, and the built-in engine's state. The engine itself loads
//! a model when a dictation needs it ([`cleanup_server::engine`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use rune::{El, Waker, row, text};

use super::super::Ui;
use super::super::controls::{ButtonKind, CardRow, Paint, button, described_row};
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::engine::{self, State};
use crate::dictation::cleanup_server::local_models::{self, LocalModel, MODELS};

/// Downloads under way, what is installed, and the engine as last seen.
#[derive(Default)]
pub(in crate::rune_ui) struct LocalModels {
    downloads: Vec<Download>,
    /// Per model in [`MODELS`]; `None` until first looked at.
    installed: Option<Vec<bool>>,
    /// The last download or delete failure, per model id.
    errors: Vec<(&'static str, String)>,
    engine: Option<State>,
}

struct Download {
    model: &'static str,
    canceled: Arc<AtomicBool>,
    bytes: Arc<AtomicU64>,
    done: Receiver<Result<(), String>>,
}

impl LocalModels {
    fn installed(&self, index: usize) -> bool {
        self.installed
            .as_ref()
            .and_then(|installed| installed.get(index).copied())
            .unwrap_or(false)
    }

    fn download(&self, model: &str) -> Option<&Download> {
        self.downloads
            .iter()
            .find(|download| download.model == model)
    }

    fn error(&self, model: &str) -> Option<&str> {
        self.errors
            .iter()
            .find(|(id, _)| *id == model)
            .map(|(_, why)| why.as_str())
    }

    /// Whether something here moves on its own (progress, loading).
    pub(super) fn busy(&self) -> bool {
        !self.downloads.is_empty() || self.engine == Some(State::Loading)
    }
}

/// While the engine loads the model, a bar with its estimated percentage
/// for the status row, in place of Test.
pub(super) fn loading_bar(ui: &Ui, p: Paint) -> Option<El> {
    if ui.output.cleanup.local.engine != Some(State::Loading) {
        return None;
    }
    let fraction = engine::load_progress()?;
    Some(
        rune_kit::Progress::new(Some(fraction.clamp(0.0, 1.0)))
            .show_value()
            .build(p.k)
            .key("loading-progress")
            .w(120.0),
    )
}

/// Whether the chosen model is downloaded, for the status row.
pub(super) fn chosen_installed(ui: &Ui) -> bool {
    let chosen = chosen(ui);
    MODELS
        .iter()
        .position(|model| model.id == chosen.id)
        .is_some_and(|index| ui.output.cleanup.local.installed(index))
}

/// The engine's state for the status row: Loaded, Loading, or not yet.
pub(super) fn engine_status(ui: &Ui, p: Paint) -> (String, rune::Color) {
    if !chosen_installed(ui) {
        return (tr("Download a model below").into(), p.muted());
    }
    match ui.output.cleanup.local.engine {
        Some(State::Ready) => (tr("Loaded").into(), p.success()),
        Some(State::Loading) => (tr("Loading the model…").into(), p.muted()),
        _ => (tr("Loads when you dictate").into(), p.muted()),
    }
}

fn chosen(ui: &Ui) -> &'static LocalModel {
    local_models::chosen(&ui.settings.dictation_processing.server.local_model)
}

/// One row per model.
pub(super) fn rows(ui: &Ui, p: Paint) -> Vec<CardRow> {
    let chosen = chosen(ui);
    MODELS
        .iter()
        .enumerate()
        .map(|(index, model)| model_row(ui, p, index, model, model.id == chosen.id).into())
        .collect()
}

fn model_row(ui: &Ui, p: Paint, index: usize, model: &'static LocalModel, chosen: bool) -> El {
    let state = &ui.output.cleanup.local;
    let key = format!("local-model-{}", model.id);
    let facts = format!(
        "{} · {} · {}",
        model.size_label(),
        model.publisher,
        model.license
    );
    let description = match state.error(model.id) {
        Some(why) => (why.to_owned(), p.danger()),
        None => (facts, p.muted()),
    };
    let controls = row().key("controls").gap(8.0).center_y();
    let controls = if state.download(model.id).is_none() {
        controls.child(ratings(p, model))
    } else {
        controls
    };
    let controls = if let Some(download) = state.download(model.id) {
        let fraction = download.bytes.load(Ordering::Relaxed) as f32 / model.bytes as f32;
        controls
            .child(
                rune_kit::Progress::new(Some(fraction.clamp(0.0, 1.0)))
                    .show_value()
                    .build(p.k)
                    .key("progress")
                    .w(120.0),
            )
            .child(button(
                p,
                &format!("{key}-cancel"),
                tr("Cancel"),
                ButtonKind::Secondary,
                move |ui, _| cancel(ui, model.id),
            ))
    } else if state.installed(index) {
        let in_use: El = if chosen {
            text(tr("In use"))
                .key("in-use")
                .font(super::super::controls::DESCRIPTION)
                .color(p.muted())
        } else {
            button(
                p,
                &format!("{key}-use"),
                tr("Use"),
                ButtonKind::Secondary,
                move |ui, _| choose(ui, model.id),
            )
        };
        controls.child(in_use).child(
            rune_kit::IconButton::new(&rune_theme_opencode::icons::TRASH)
                .variant(rune_kit::IconButtonVariant::Ghost)
                .control_size(rune_kit::ControlSize::Regular)
                .label(tr("Delete"))
                .on_press(move |ui: &mut Ui| delete(ui, index))
                .build(p.k)
                .key(format!("{key}-delete")),
        )
    } else {
        controls.child(button(
            p,
            &format!("{key}-download"),
            tr("Download"),
            ButtonKind::Secondary,
            move |ui, _| start_download(ui, model),
        ))
    };
    described_row(p, &key, model.name, Some(description), controls)
}

/// Speed and quality, five stars each, labels beside them.
fn ratings(p: Paint, model: &LocalModel) -> El {
    const LINE: f32 = 15.0;
    let label = |copy: &'static str| {
        text(tr(copy))
            .font(super::super::controls::DESCRIPTION)
            .color(p.muted())
            .h(LINE)
    };
    let stars = |key: &'static str, copy: &'static str, filled: u8| {
        row()
            .key(key)
            .role("image")
            .label(format!("{} {filled}/5", tr(copy)))
            .h(LINE)
            .gap(1.0)
            .center_y()
            .children((0..5u8).map(|index| {
                let (glyph, color) = if index < filled {
                    (&rune_kit::glyphs::STAR_FILLED, p.text())
                } else {
                    (&rune_kit::glyphs::STAR, p.muted())
                };
                rune::icon(glyph, 10.0).color(color)
            }))
    };
    row()
        .key("ratings")
        .gap(6.0)
        .child(
            rune::col()
                .items_end()
                .child(label("Speed"))
                .child(label("Quality")),
        )
        .child(
            rune::col()
                .child(stars("speed", "Speed", model.speed))
                .child(stars("quality", "Quality", model.quality)),
        )
}

fn choose(ui: &mut Ui, id: &'static str) {
    ui.update(|settings| settings.dictation_processing.server.local_model = id.into());
}

fn start_download(ui: &mut Ui, model: &'static LocalModel) {
    if !ui.live() || ui.output.cleanup.local.download(model.id).is_some() {
        return;
    }
    let canceled = Arc::new(AtomicBool::new(false));
    let bytes = Arc::new(AtomicU64::new(0));
    let (sender, done) = channel();
    let waker: Option<Waker> = ui.modes.waker.clone();
    let (thread_canceled, thread_bytes) = (canceled.clone(), bytes.clone());
    let started = std::thread::Builder::new()
        .name("hex-cleanup-model-download".into())
        .spawn(move || {
            let result = local_models::download(model, &thread_canceled, &thread_bytes)
                .map(|_| ())
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
            if let Some(waker) = &waker {
                waker();
            }
        });
    if started.is_ok() {
        let local = &mut ui.output.cleanup.local;
        local.errors.retain(|(id, _)| *id != model.id);
        local.downloads.push(Download {
            model: model.id,
            canceled,
            bytes,
            done,
        });
    }
}

fn cancel(ui: &mut Ui, id: &str) {
    if let Some(download) = ui.output.cleanup.local.download(id) {
        download.canceled.store(true, Ordering::Release);
    }
}

fn delete(ui: &mut Ui, index: usize) {
    let model = &MODELS[index];
    let local = &mut ui.output.cleanup.local;
    local.errors.retain(|(id, _)| *id != model.id);
    if let Err(error) = model.remove() {
        local.errors.push((model.id, format!("{error:#}")));
    }
    local.installed = None;
}

/// Takes finished downloads, looks at what is installed, and follows the
/// engine; true when what shows changed.
pub(super) fn poll(ui: &mut Ui) -> bool {
    let live = ui.live();
    let local = &mut ui.output.cleanup.local;
    let mut changed = false;
    let mut finished = Vec::new();
    local
        .downloads
        .retain(|download| match download.done.try_recv() {
            Ok(result) => {
                finished.push((
                    download.model,
                    download.canceled.load(Ordering::Acquire),
                    result,
                ));
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                finished.push((download.model, true, Ok(())));
                false
            }
        });
    for (model, canceled, result) in finished {
        changed = true;
        local.installed = None;
        if let Err(why) = result
            && !canceled
        {
            local
                .errors
                .push((model, tr_fill("Download failed: {}", &why)));
        }
    }
    if !local.downloads.is_empty() {
        changed = true;
    }
    if local.installed.is_none() {
        local.installed = Some(MODELS.iter().map(LocalModel::installed).collect());
        changed = true;
    }
    if live {
        let engine = Some(engine::state());
        // While loading, its percentage moves.
        if local.engine != engine || engine == Some(State::Loading) {
            local.engine = engine;
            changed = true;
        }
    }
    changed
}

/// The fixture's built-in models: the first halfway downloaded, the
/// others downloaded, with Use and their trash buttons.
pub(super) fn fixture(local: &mut LocalModels) {
    let (sender, done) = channel::<Result<(), String>>();
    // Kept open, so the fixture's download never finishes.
    std::mem::forget(sender);
    local.installed = Some((0..MODELS.len()).map(|index| index > 0).collect());
    local.downloads.push(Download {
        model: MODELS[0].id,
        canceled: Arc::new(AtomicBool::new(false)),
        bytes: Arc::new(AtomicU64::new(MODELS[0].bytes / 2)),
        done,
    });
}
