//! Labs › Clean up on your own server › Built in, in Output: the models Hex
//! downloads for cleanup, each with Download (and its progress and Cancel),
//! Use, and Delete, and the built-in engine's state. The engine itself loads
//! a model when a dictation needs it ([`cleanup_server::engine`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use rune::{El, Waker, col, row, text};

use super::super::Ui;
use super::super::controls::{
    ButtonKind, CardRow, DESCRIPTION, Paint, PushSize, button, copy_row_tagged,
};
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::engine::{self, State};
use crate::dictation::cleanup_server::local_models::{self, LocalModel, MODELS};

/// One segment of a speed or quality meter.
const SEGMENT: (f32, f32) = (7.0, 6.0);
/// A file fact's tag.
const CHIP_H: f32 = 18.0;

/// Downloads under way, what is installed, and the engine as last seen.
#[derive(Default)]
pub(in crate::rune_ui) struct LocalModels {
    downloads: Vec<Download>,
    /// Per model in [`MODELS`]; `None` until first looked at.
    installed: Option<Vec<bool>>,
    /// The last download or delete failure, per model id.
    errors: Vec<(&'static str, String)>,
    engine: Option<State>,
    /// The model Use last chose, loaded at once and again whenever the
    /// engine is free, until it runs.
    wanted: Option<&'static LocalModel>,
    /// The model whose trash was pressed, asking to confirm.
    confirm_delete: Option<&'static str>,
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

/// Whether the built-in engine is loading a model, which that model's row
/// shows.
pub(super) fn loading(ui: &Ui) -> bool {
    ui.output.cleanup.local.engine == Some(State::Loading)
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
    let facts = row()
        .key("facts")
        .gap(6.0)
        .center_y()
        .flex_wrap()
        .child(chip(
            p,
            "quantization",
            model.quantization().to_owned(),
            true,
        ))
        .children(
            model
                .bits()
                .map(|bits| chip(p, "bits", tr_fill("{}-bit", &bits.to_string()), false)),
        )
        .child(chip(p, "size", model.size_label(), false))
        // The licence stays a hover away.
        .child(
            text(model.publisher)
                .key("publisher")
                .font(DESCRIPTION)
                .color(p.muted())
                .help(model.license),
        );
    let description = match state.error(model.id) {
        Some(why) => text(why)
            .key("description")
            .font(DESCRIPTION)
            .color(p.danger())
            .wrap(),
        None => col()
            .key("description")
            .gap(3.0)
            .child(
                row()
                    .key("meters")
                    .gap(16.0)
                    .center_y()
                    .flex_wrap()
                    .child(meter(
                        p,
                        "speed",
                        tr("Speed"),
                        model.speed,
                        speed_label(model.speed),
                        p.k.c.success.fg,
                    ))
                    .child(meter(
                        p,
                        "quality",
                        tr("Quality"),
                        model.quality,
                        quality_label(model.quality),
                        p.k.c.info.fg,
                    )),
            )
            .child(facts),
    };
    let controls = row().key("controls").gap(8.0).center_y();
    let controls = if state.confirm_delete == Some(model.id) {
        controls
            .child(button(
                p,
                &format!("{key}-keep"),
                tr("Cancel"),
                ButtonKind::Secondary,
                |ui, _| ui.output.cleanup.local.confirm_delete = None,
            ))
            .child(button(
                p,
                &format!("{key}-delete-confirm"),
                &tr_fill("Delete {}", &model.size_label()),
                ButtonKind::Danger,
                move |ui, _| delete(ui, index),
            ))
    } else if let Some(download) = state.download(model.id) {
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
        let loading = engine::load_progress()
            .filter(|(id, _)| *id == model.id && state.engine == Some(State::Loading));
        let in_use: El = if let Some((_, fraction)) = loading {
            rune_kit::Button::new(tr_fill(
                "Loading {}",
                &format!("{}%", (fraction * 100.0).floor() as u32),
            ))
            .variant(rune_kit::ButtonVariant::Loading)
            .build_push(p.k)
            .key(format!("{key}-loading"))
        } else if chosen {
            in_use_pill(p)
        } else {
            button(
                p,
                &format!("{key}-use"),
                tr("Use"),
                ButtonKind::Secondary,
                move |ui, _| choose(ui, model),
            )
        };
        controls.child(in_use).child(
            rune_kit::IconButton::new(&rune_theme_opencode::icons::TRASH)
                .variant(rune_kit::IconButtonVariant::Ghost)
                .control_size(rune_kit::ControlSize::Regular)
                .label(tr("Delete"))
                .on_press(move |ui: &mut Ui| {
                    ui.output.cleanup.local.confirm_delete = Some(model.id)
                })
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
    // Its full name, as LM Studio shows a model; who converted it is in
    // the line below.
    let full_name = model
        .repository()
        .rsplit_once('/')
        .map_or(model.repository(), |(_, name)| name);
    let source = text(full_name)
        .key("source")
        .font(DESCRIPTION)
        .color(p.faint());
    copy_row_tagged(
        p,
        &key,
        None,
        model.name,
        Some(source),
        Some(description),
        controls,
        false,
    )
}

/// A fact about the file as a rounded tag, in the code font when `code`.
fn chip(p: Paint, key: &str, label: String, code: bool) -> El {
    let font = if code {
        rune::TextStyle {
            size: 11.0,
            line_height: 14.0,
            ..p.k.ty.mono
        }
    } else {
        DESCRIPTION
    };
    row()
        .key(key)
        .h(CHIP_H)
        .px(7.0)
        .center_y()
        .radius(CHIP_H / 2.0)
        .border(0.5, p.k.c.border_base)
        .child(text(label).font(font).color(p.muted()))
}

/// A rating out of five as five segments, lit in `color` up to `level`,
/// after its name; `word` says it to a screen reader.
fn meter(p: Paint, key: &str, name: &str, level: u8, word: &str, color: rune::Color) -> El {
    let segments = row()
        .key("segments")
        .gap(2.0)
        .children((1..=5).map(|segment| {
            rune::el()
                .size(SEGMENT.0, SEGMENT.1)
                .radius(2.0)
                .fill(if segment <= level {
                    color
                } else {
                    p.faint().alpha(0.3)
                })
        }));
    row()
        .key(key)
        .gap(6.0)
        .center_y()
        .role("meter")
        .label(format!("{name}: {word}"))
        .child(text(name).font(DESCRIPTION).color(p.muted()))
        .child(segments)
}

/// The chosen model's mark: a pressed-looking success pill with a check.
fn in_use_pill(p: Paint) -> El {
    let c = &p.k.c;
    row()
        .key("in-use")
        .role("status")
        .label(tr("In use"))
        .h(28.0)
        .px(10.0)
        .gap(5.0)
        .center_y()
        .radius(p.k.radius.sm)
        .fill(c.success.bg)
        .border(0.5, c.success.border)
        .child(rune::icon(&rune_kit::glyphs::CHECK, 12.0).color(c.success.fg))
        .child(
            text(tr("In use"))
                .font(p.k.ty.ui_medium)
                .color(c.success.fg),
        )
}

/// A model's speed in a word, from its 1–5 rating.
fn speed_label(speed: u8) -> &'static str {
    match speed {
        5.. => tr("Fastest"),
        4 => tr("Fast"),
        _ => tr("Slower"),
    }
}

/// A model's quality in a word, from its 1–5 rating.
fn quality_label(quality: u8) -> &'static str {
    match quality {
        5.. => tr("Best"),
        4 => tr("Good"),
        _ => tr("Basic"),
    }
}

/// Use: saves the choice and loads the model at once, so it is ready (and
/// shows loading) before the next dictation.
fn choose(ui: &mut Ui, model: &'static LocalModel) {
    ui.update(|settings| settings.dictation_processing.server.local_model = model.id.into());
    if ui.live() {
        ui.output.cleanup.local.wanted = Some(model);
        engine::warm(model);
    }
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
    local.confirm_delete = None;
    if local.wanted.is_some_and(|wanted| wanted.id == model.id) {
        local.wanted = None;
    }
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
    if live && let Some(wanted) = local.wanted {
        // A choice made while another model was loading loads once it is done.
        if engine::running_model() == Some(wanted.id) {
            local.wanted = None;
        } else if engine::state() != State::Loading {
            engine::warm(wanted);
        }
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
    // The last asks to confirm its delete.
    local.confirm_delete = MODELS.last().map(|model| model.id);
    local.downloads.push(Download {
        model: MODELS[0].id,
        canceled: Arc::new(AtomicBool::new(false)),
        bytes: Arc::new(AtomicU64::new(MODELS[0].bytes / 2)),
        done,
    });
}
