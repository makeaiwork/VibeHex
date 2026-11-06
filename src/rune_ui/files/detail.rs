//! A file's card: its tabs (the versions of its text and the jobs that will
//! add more), the open version with its line, actions, and text, and the
//! request field under it.

use rune::{El, TextEdit, Tx, col, icon, row, text};
use rune_kit::TextArea;

use super::super::Ui;
use super::super::controls::{ButtonKind, DESCRIPTION, Paint, button};
use super::super::list_detail as ld;
use super::actions;
use super::tabs;
use super::{Armed, Tab, failure, stage_label, submit};
use crate::desktop::i18n::{tr, tr_fill, tr_named};
use crate::files::rewrite::DraftKind;
use crate::files::store::{self, FileRecord, Version, VersionKind};
use crate::files::text as file_text;
use crate::files::worker::{self, Draft, Job, Stage, Task};

static CLOCK_ICON: rune::Icon = rune::Icon {
    name: "hex-files-clock",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M8 4.5V8l2.3 1.5" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#,
};

static MIC_ICON: rune::Icon = rune::Icon {
    name: "hex-files-mic",
    view: 16.0,
    body: r#"<rect x="5.5" y="1.8" width="5" height="8" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.2" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#,
};

/// The tab shown: the chosen one while it exists, else the first cleanup,
/// else the first version, else a job's.
pub(super) fn current(ui: &Ui, record: &FileRecord) -> Option<Tab> {
    let pane = &ui.files;
    match pane.tab {
        Some(Tab::Version(id)) if record.version(id).is_some() => return pane.tab,
        Some(Tab::Task(id)) if pane.tasks_of(&record.id).any(|task| task.id == id) => {
            return pane.tab;
        }
        _ => {}
    }
    record
        .versions
        .iter()
        .find(|version| version.kind == VersionKind::Cleanup)
        .or(record.versions.first())
        .map(|version| Tab::Version(version.id))
        .or_else(|| {
            pane.tasks_of(&record.id)
                .next()
                .map(|task| Tab::Task(task.id))
        })
}

/// Applies `change` to the shown record and, outside scenes, the stored one.
pub(super) fn change(ui: &mut Ui, id: &str, change: impl Fn(&mut FileRecord)) {
    if let Some(listed) = ui.files.records.iter_mut().find(|l| l.record.id == id) {
        change(&mut listed.record);
    }
    if !ui.files.fixture
        && let Err(error) = store::update(id, change)
    {
        ui.files.error = Some(format!("{error:#}"));
    }
}

/// "Original" and "Cleanup" in the interface's language; a request's own
/// name as typed.
pub(super) fn tab_label(name: &str) -> String {
    for (english, shown) in [("Original", tr("Original")), ("Cleanup", tr("Cleanup"))] {
        if name == english {
            return shown.into();
        }
        if let Some(rest) = name.strip_prefix(english)
            && rest.starts_with(" (")
        {
            return format!("{shown}{rest}");
        }
    }
    name.into()
}

/// Whether Original tabs show their times.
fn timestamps(ui: &Ui) -> bool {
    ui.files
        .timestamps
        .unwrap_or(ui.settings.files.show_timestamps)
}

/// What a version reads as: its Markdown and its plain text.
pub(super) fn texts(ui: &Ui, version: &Version) -> (String, String) {
    if version.kind == VersionKind::Original && !version.edited {
        let shown = if timestamps(ui) {
            file_text::timestamped(&version.segments)
        } else {
            file_text::plain(&version.segments)
        };
        return (shown.clone(), shown);
    }
    (
        version.text.clone(),
        file_text::strip_markdown(&version.text),
    )
}

pub(super) fn view(ui: &Ui, p: Paint, record: &FileRecord, layer: &mut Vec<El>) -> El {
    let pane = &ui.files;
    let tab = current(ui, record);
    let version = match tab {
        Some(Tab::Version(id)) => record.version(id),
        _ => None,
    };
    let editing = version.filter(|version| pane.editing == Some(version.id));
    let mut actions = Vec::new();
    if let Some(version) = version
        && editing.is_none()
    {
        actions.push(actions::copy_menu(ui, p, version, layer));
        let id = record.id.clone();
        let (markdown, _) = texts(ui, version);
        let version_id = version.id;
        actions.push(ld::ghost_icon(
            p,
            "file-edit",
            ld::EDIT_ICON,
            tr("Edit"),
            move |ui: &mut Ui, _| {
                if ui.files.selected.as_deref() == Some(id.as_str()) {
                    ui.files.edit = TextEdit::new(markdown.clone());
                    ui.files.editing = Some(version_id);
                    ui.open = None;
                }
            },
        ));
    }
    if editing.is_none() {
        actions.push(actions::more_menu(ui, p, record, version, layer));
    }
    let listed = pane.records.iter().find(|l| l.record.id == record.id);
    let mut meta = Vec::new();
    if let Some(listed) = listed {
        meta.push(ld::meta_text(
            p,
            &format!("{}, {}", super::day_label(&listed.day), listed.clock),
        ));
    }
    let with_icon = |key: &str, glyph: &'static rune::Icon, words: String| {
        row()
            .key(key)
            .gap(4.0)
            .center_y()
            .shrink()
            .child(icon(glyph, 12.0).color(p.muted()))
            .child(ld::meta_text(p, &words))
    };
    if record.duration_ms > 0 {
        meta.push(with_icon(
            "length",
            &CLOCK_ICON,
            file_text::clock(record.duration_ms),
        ));
    }
    if let Some(first) = record.versions.first() {
        meta.push(with_icon("model", &MIC_ICON, first.model.clone()));
    }
    let title = text(record.name.clone())
        .key("detail-title")
        .font(p.k.ty.body_medium)
        .color(p.text())
        .wrap_anywhere()
        .max_lines(2);
    let header = ld::card_header(title, meta, actions);

    let mut body = col().key("file-body").gap(14.0).pb(8.0);
    let tabs = tabs::strip(ui, p, record, tab, layer);
    let tasks: Vec<&Task> = pane.tasks_of(&record.id).collect();
    if let Some(tabs) = tabs {
        body = body.child(tabs);
    }
    if let Some(armed) = confirm(p, pane.armed.as_ref(), record) {
        body = body.child(armed);
    }
    if !pane.fixture
        && let Some(error) = worker::error(&record.id)
    {
        let id = record.id.clone();
        body = body.child(super::super::lists::dismissible_error_panel(
            p,
            "job-error",
            tr("That didn't work."),
            &error,
            move |ui: &mut Ui| {
                worker::dismiss_error(&id);
                ui.files.stale = true;
            },
        ));
    }
    let mut ask = None;
    match (tab, version) {
        (Some(Tab::Task(id)), _) => {
            if let Some(task) = tasks.iter().find(|task| task.id == id) {
                body = body.children(task_line(p, record, task));
                body = body.child(progress(p, record, task));
                body = body.children(task.draft.as_ref().map(|d| draft(p, d)));
                if task.kind != VersionKind::Original {
                    let name = task.name.clone().unwrap_or_else(|| tr("Cleanup").into());
                    ask = Some(actions::ask_area(ui, p, Some(&name), layer));
                }
            }
        }
        (_, Some(version)) => {
            body = body.child(version_line(ui, p, record, version, layer));
            if let Some(again) = pane
                .again
                .as_ref()
                .filter(|again| again.record == record.id)
                .filter(|_| version.kind == VersionKind::Original)
            {
                body = body.child(actions::again_row(ui, p, again, layer));
            }
            if let Some(parts) = version.parts.filter(|_| !version.note_closed) {
                body = body.child(steps_note(p, record, version.id, parts));
            }
            if let Some(failed) = &version.failed {
                body = body.child(
                    text(tr_fill("Some parts kept their text: {}", failed))
                        .key("failed")
                        .font(DESCRIPTION)
                        .color(p.warning())
                        .wrap(),
                );
            }
            if editing.is_some() {
                body = body.child(editor(p, ui, record, version.id));
            } else if version.kind == VersionKind::Request {
                let typed = tabs::typed_prompt(ui, version).is_some();
                body = body.child(super::session::chat(ui, p, record, version, typed));
                let target = super::composer::Target::Session(version.id);
                let running = super::composer::running(ui, target);
                ask = Some(actions::composer_area(super::composer::field(
                    ui,
                    p,
                    target,
                    tr("Ask a follow-up"),
                    running,
                    None,
                    layer,
                )));
            } else {
                body = body.child(content(ui, p, version));
                ask = Some(actions::ask_area(ui, p, None, layer));
            }
        }
        _ => {
            if let Some(error) = &record.error {
                body = body.child(failure(p, record, error));
            } else if let Some(task) = tasks.first() {
                body = body.child(progress(p, record, task));
            } else {
                // Hex quit while it was transcribing.
                let stopped = tr("The transcription stopped before it finished.");
                body = body.child(failure(p, record, stopped));
            }
        }
    }
    let detail = col().key("file-detail").grow(1.0).shrink().child(header);
    match ask {
        Some(ask) => detail.child(floating(p, pane, body, ask)),
        None => detail.child(ld::card_scroll("file-text", body)),
    }
}

/// Room left under the text for the field, past its own height.
const FIELD_CLEARANCE: f32 = 12.0;
/// Before the field is first measured.
const FIELD_GUESS: f32 = 110.0;

/// The text with the field floating over its bottom: the text scrolls on
/// under it, fading out as it nears the field, and once text is under it the
/// field lifts off with a shadow.
fn floating(p: Paint, pane: &super::FilesPane, body: El, ask: El) -> El {
    let field_h = if pane.composer_h > 0.0 {
        pane.composer_h
    } else {
        FIELD_GUESS
    };
    let room = field_h + FIELD_CLEARANCE;
    let bg = p.k.c.bg_base;
    let body = body.child(rune::el().key("field-room").h(room));
    let text = ld::card_scroll("file-text", body)
        .on_at_bottom(|ui: &mut Ui, at: bool| ui.files.text_under = !at);
    let veil = rune::el()
        .key("field-veil")
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .h(room + 32.0)
        .linear_gradient(
            std::f32::consts::PI,
            [
                (0.0, bg.alpha(0.0)),
                (0.3, bg.alpha(0.7)),
                (0.55, bg.alpha(0.85)),
                (1.0, bg.alpha(0.95)),
            ],
        );
    let ask =
        ask.pin(Some(0.0), None, Some(0.0), Some(0.0))
            .on_layout(|ui: &mut Ui, rect: [f32; 4]| {
                if (ui.files.composer_h - rect[3]).abs() > 0.5 {
                    ui.files.composer_h = rect[3];
                }
            });
    col()
        .key("file-stage")
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .child(text)
        .child(veil)
        .child(ask)
}

/// The open version's line: where it came from, and what it can do again.
fn version_line(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    version: &Version,
    layer: &mut Vec<El>,
) -> El {
    let window = version.window_k.map(|k| k.to_string());
    let mut words = match version.kind {
        VersionKind::Original => String::new(),
        VersionKind::Cleanup => match &window {
            Some(window) => tr_named(
                "{model} in an {window}K window",
                &[("model", &version.model), ("window", window)],
            ),
            None => version.model.clone(),
        },
        VersionKind::Request => {
            let from = version
                .from
                .and_then(|from| record.version(from))
                .map_or_else(|| tr("a deleted tab").to_string(), |v| tab_label(&v.name));
            match &window {
                Some(window) => tr_named(
                    "Made from {from} by {model}, {window}K window",
                    &[
                        ("from", &from),
                        ("model", &version.model),
                        ("window", window),
                    ],
                ),
                None => tr_named(
                    "Made from {from} by {model}",
                    &[("from", &from), ("model", &version.model)],
                ),
            }
        }
    };
    if version.edited {
        if words.is_empty() {
            words = tr("Edited").into();
        } else {
            words = format!("{words}, {}", tr("edited"));
        }
    }
    let mut line = row().key("version-line").gap(6.0).pr(1.0).center_y();
    if let Some(name) = tabs::tag(version) {
        line = line.child(
            row()
                .key("tag")
                .h(20.0)
                .px(7.0)
                .center_y()
                .radius(p.k.radius.sm)
                .fill(p.k.c.layer_02)
                .border(p.hair, p.k.c.border_weak)
                .child(
                    text(name)
                        .key("name")
                        .font(p.k.ty.small_medium)
                        .color(p.text())
                        .max_w(180.0)
                        .truncate(),
                ),
        );
    }
    if version.kind == VersionKind::Original && !version.edited {
        let selected = usize::from(!timestamps(ui));
        line = line.child(timestamp_switch(p, selected == 0));
    }
    if !words.is_empty() {
        line = line.child(
            text(words)
                .key("version-words")
                .font(DESCRIPTION)
                .color(p.muted())
                .shrink()
                .truncate(),
        );
    }
    line = line.child(rune::el().key("spacer").grow(1.0));
    if let Some(position) = tabs::position(ui, record, version) {
        line = line.child(
            text(position)
                .key("position")
                .font(DESCRIPTION)
                .color(p.faint()),
        );
    }
    match version.kind {
        VersionKind::Original => {
            let cleaned = record
                .versions
                .iter()
                .any(|other| other.kind == VersionKind::Cleanup && other.from == Some(version.id));
            if !cleaned {
                let (id, from) = (record.id.clone(), version.id);
                line = line.child(button(
                    p,
                    "file-clean-up",
                    tr("Clean up"),
                    ButtonKind::Secondary,
                    move |ui: &mut Ui, _| {
                        if let Some(task) = submit(ui, &id, Job::Cleanup { from, engine: None }) {
                            ui.files.tab = Some(Tab::Task(task));
                        }
                    },
                ));
            }
            line = line.child(actions::again_menu(ui, p, record, layer));
        }
        VersionKind::Cleanup => {
            line = line.child(actions::redo_menu(ui, p, record, version, layer))
        }
        VersionKind::Request => {}
    }
    line
}

/// Why a version went in three steps, until closed.
fn steps_note(p: Paint, record: &FileRecord, version: u32, parts: usize) -> El {
    let id = record.id.clone();
    row()
        .key("steps-note")
        .gap(8.0)
        .px(12.0)
        .py(8.0)
        .radius(p.k.radius.md)
        .fill(p.k.c.layer_02)
        .child(
            text(tr_fill(
                "The text is longer than the model's window, so it went in 3 steps over {} parts. A model with a large window reads it whole: Codex, OpenRouter or OpenCode.",
                &parts.to_string(),
            ))
            .key("words")
            .font(DESCRIPTION)
            .color(p.muted())
            .grow(1.0)
            .shrink()
            .wrap(),
        )
        .child(ld::ghost_icon(
            p,
            "steps-note-close",
            p.k.glyph.close,
            tr("Close"),
            move |ui: &mut Ui, _| {
                change(ui, &id, |record| {
                    if let Some(version) = record.version_mut(version) {
                        version.note_closed = true;
                    }
                });
            },
        ))
}

/// The version's text: an Original as timed or plain paragraphs, anything
/// else as Markdown.
fn content(ui: &Ui, p: Paint, version: &Version) -> El {
    if version.kind == VersionKind::Original && !version.edited {
        let paragraphs = file_text::paragraphs(&version.segments);
        if paragraphs.is_empty() {
            return text(tr("No speech was found."))
                .key("no-speech")
                .font(DESCRIPTION)
                .color(p.faint());
        }
        let shown = timestamps(ui);
        let mut lines = col().key("paragraphs").gap(12.0);
        for (index, paragraph) in paragraphs.into_iter().enumerate() {
            let words = text(paragraph.text)
                .key("text")
                .font(p.k.ty.ui)
                .color(p.reading())
                .grow(1.0)
                .shrink()
                .wrap();
            lines = lines.child(if shown {
                row()
                    .key(format!("paragraph{index}"))
                    .gap(12.0)
                    .child(
                        text(file_text::clock(paragraph.start_ms))
                            .key("clock")
                            .font(p.k.ty.text_xsmall_medium)
                            .color(p.faint())
                            .w(40.0)
                            .pt(1.0),
                    )
                    .child(words)
            } else {
                row().key(format!("paragraph{index}")).child(words)
            });
        }
        return lines;
    }
    if version.text.trim().is_empty() {
        return text(tr("This version is empty."))
            .key("empty")
            .font(DESCRIPTION)
            .color(p.faint());
    }
    rune_kit::Markdown::new(version.text.clone())
        .style(
            rune_kit::MarkdownStyle::from_kit(p.k)
                .body_font(p.k.ty.ui)
                .text(p.reading()),
        )
        .selectable(true)
        .tasks_read_only()
        .build(p.k)
        .key("markdown")
}

/// Editing a version by hand, with Save and Cancel.
fn editor(p: Paint, ui: &Ui, record: &FileRecord, version: u32) -> El {
    let id = record.id.clone();
    col()
        .key("file-editor")
        .gap(8.0)
        .child(
            TextArea::new(&ui.files.edit)
                .rows(14)
                .max_height(440.0)
                .width(None)
                .on_edit(|ui: &mut Ui| &mut ui.files.edit)
                .build(p.k)
                .key("file-edit-text"),
        )
        .child(
            row()
                .key("edit-actions")
                .gap(8.0)
                .child(button(
                    p,
                    "file-edit-save",
                    tr("Save"),
                    ButtonKind::Primary,
                    move |ui: &mut Ui, _| {
                        let edited = ui.files.edit.text.clone();
                        change(ui, &id, |record| {
                            if let Some(version) = record.version_mut(version) {
                                version.text = edited.clone();
                                version.edited = true;
                            }
                        });
                        ui.files.editing = None;
                    },
                ))
                .child(button(
                    p,
                    "file-edit-cancel",
                    tr("Cancel"),
                    ButtonKind::Secondary,
                    |ui: &mut Ui, _| ui.files.editing = None,
                )),
        )
}

/// A job at work: its stage, a bar, and Stop; a request in three steps
/// also lists the steps.
fn progress(p: Paint, record: &FileRecord, task: &Task) -> El {
    let c = &p.k.c;
    let percent = (task.fraction * 100.0).round() as u32;
    let title = match (task.stage, task.parts) {
        (Stage::Working, Some(_)) => tr("Working in 3 steps").to_string(),
        _ => stage_label(task),
    };
    let detail = match task.stage {
        Stage::Transcribing if record.duration_ms > 0 => Some(tr_named(
            "{done} of {total}",
            &[
                (
                    "done",
                    &file_text::clock((record.duration_ms as f32 * task.fraction) as u64),
                ),
                ("total", &file_text::clock(record.duration_ms)),
            ],
        )),
        Stage::Queued => Some(tr("Waits for the jobs before it.").to_string()),
        _ => None,
    };
    let id = task.id;
    let stop = actions::quiet_button(
        p,
        "file-stop",
        None,
        if task.stage == Stage::Queued {
            tr("Remove")
        } else {
            tr("Stop")
        },
        None,
        false,
        move |ui: &mut Ui| {
            if !ui.files.fixture {
                worker::cancel(id);
            }
            ui.files.stale = true;
        },
    );
    let mut card = col()
        .key("file-progress")
        .gap(9.0)
        .px(14.0)
        .py(12.0)
        .radius(9.0)
        .fill(c.layer_01)
        .box_shadow(rune::Shadow {
            spread: 0.5,
            color: c.border_base,
            ..Default::default()
        })
        .child(
            row()
                .key("head")
                .gap(8.0)
                .center_y()
                .child(
                    text(title)
                        .key("title")
                        .font(p.k.ty.ui_medium)
                        .color(p.text())
                        .shrink()
                        .truncate(),
                )
                .children((task.stage != Stage::Queued).then(|| {
                    text(format!("{percent}%"))
                        .key("percent")
                        .font(p.k.ty.text_xsmall_medium)
                        .color(p.muted())
                }))
                .child(rune::el().key("spacer").grow(1.0))
                .child(stop),
        );
    if task.stage != Stage::Queued {
        card = card.child(ld::thin_bar(p, task.fraction));
    }
    if let Some(detail) = detail {
        card = card.child(
            text(detail)
                .key("detail")
                .font(DESCRIPTION)
                .color(p.muted()),
        );
    }
    if let (Stage::Working, Some(parts)) = (task.stage, task.parts) {
        card = card.child(steps(p, task.fraction, parts));
    }
    card
}

/// What a running cleanup or request has written so far, as Markdown that
/// is still arriving, with a cursor at its end.
fn draft(p: Paint, draft: &Draft) -> El {
    let label = match draft.kind {
        DraftKind::Summary(part) => tr_fill("Summary of part {} so far:", &part.to_string()),
        DraftKind::Outline => tr("Outline so far:").to_string(),
        DraftKind::Result => tr("Text so far:").to_string(),
    };
    let mut source = String::new();
    if draft.cut {
        source.push_str("…\n\n");
    }
    source.push_str(draft.text.trim_end());
    source.push('▌');
    col()
        .key("file-draft")
        .gap(8.0)
        .pt(4.0)
        .child(text(label).key("label").font(p.k.ty.ui).color(p.muted()))
        .child(
            rune_kit::Markdown::new(source)
                .style(
                    rune_kit::MarkdownStyle::from_kit(p.k)
                        .body_font(p.k.ty.ui)
                        .text(p.muted()),
                )
                .streaming(true)
                .build(p.k)
                .key("markdown"),
        )
}

/// What a running cleanup or request is made from and runs on, as its
/// version's line will read.
fn task_line(p: Paint, record: &FileRecord, task: &Task) -> Option<El> {
    let model = task.model.as_deref()?;
    let window = task.window_k.map(|k| k.to_string());
    let words = match task.kind {
        VersionKind::Cleanup => match &window {
            Some(window) => tr_named(
                "{model} in an {window}K window",
                &[("model", model), ("window", window)],
            ),
            None => model.to_string(),
        },
        _ => {
            let from = task
                .from
                .and_then(|from| record.version(from))
                .map_or_else(|| tr("a deleted tab").to_string(), |v| tab_label(&v.name));
            match &window {
                Some(window) => tr_named(
                    "Made from {from} by {model}, {window}K window",
                    &[("from", &from), ("model", model), ("window", window)],
                ),
                None => tr_named(
                    "Made from {from} by {model}",
                    &[("from", &from), ("model", model)],
                ),
            }
        }
    };
    Some(
        text(words)
            .key("task-line")
            .font(DESCRIPTION)
            .color(p.muted())
            .truncate(),
    )
}

/// One step's mark: a green tick when done, a turning ring while it runs,
/// an empty ring until then.
fn step_mark(p: Paint, order: std::cmp::Ordering) -> El {
    let c = &p.k.c;
    match order {
        std::cmp::Ordering::Less => row()
            .key("mark")
            .size(14.0, 14.0)
            .center()
            .radius(7.0)
            .fill(p.success())
            .child(
                icon(p.k.glyph.check_small, 12.0)
                    .key("tick")
                    .color(c.bg_base),
            ),
        std::cmp::Ordering::Equal => rune_kit::Loader::new().size(14.0).build(p.k).key("mark"),
        std::cmp::Ordering::Greater => rune::el()
            .key("mark")
            .size(14.0, 14.0)
            .radius(7.0)
            .border(1.5, c.border_strong),
    }
}

/// The three steps, each with its mark; the ones still to come in faint ink.
fn steps(p: Paint, fraction: f32, parts: usize) -> El {
    let calls = (parts * 2 + 1) as f32;
    let done = (fraction * calls).round() as usize;
    let step = if done < parts {
        0
    } else if done == parts {
        1
    } else {
        2
    };
    let labels = [
        tr_named(
            "Short summary of each part ({done} of {parts})",
            &[
                ("done", &done.min(parts).to_string()),
                ("parts", &parts.to_string()),
            ],
        ),
        tr("Overall outline from the summaries").to_string(),
        tr("Each part with the outline, then joined").to_string(),
    ];
    let font = rune::TextStyle {
        size: 12.5,
        ..p.k.ty.ui
    };
    let mut list = col().key("steps").gap(9.0).child(
        text(tr_fill(
            "The text is longer than the model's window, so it goes in {} parts.",
            &parts.to_string(),
        ))
        .key("why")
        .font(DESCRIPTION)
        .color(p.muted())
        .wrap(),
    );
    for (index, label) in labels.into_iter().enumerate() {
        let order = index.cmp(&step);
        let ink = if order == std::cmp::Ordering::Greater {
            p.faint()
        } else {
            p.text()
        };
        list = list.child(
            row()
                .key(format!("step{index}"))
                .gap(9.0)
                .center_y()
                .child(step_mark(p, order))
                .child(text(label).key("label").font(font).color(ink).wrap()),
        );
    }
    list
}

/// The delete confirmation under the tabs while one is armed.
fn confirm(p: Paint, armed: Option<&Armed>, record: &FileRecord) -> Option<El> {
    let (label, armed) = match armed? {
        Armed::Version(id, version) if *id == record.id => (
            tr("Delete this version"),
            Armed::Version(id.clone(), *version),
        ),
        Armed::File(id) if *id == record.id => {
            (tr("Remove from the list"), Armed::File(id.clone()))
        }
        _ => return None,
    };
    let note = match armed {
        Armed::Version(..) => tr("The other tabs stay."),
        Armed::File(_) => tr("Its texts are deleted; the file itself stays where it is."),
    };
    Some(
        col()
            .key("delete-confirm")
            .gap(8.0)
            .child(text(note).key("note").font(DESCRIPTION).color(p.muted()))
            .child(
                row()
                    .key("buttons")
                    .gap(8.0)
                    .child(
                        button(
                            p,
                            "file-confirm-delete",
                            label,
                            ButtonKind::Danger,
                            move |ui: &mut Ui, _| delete(ui, armed.clone()),
                        )
                        .default_button(),
                    )
                    .child(button(
                        p,
                        "file-cancel-delete",
                        tr("Cancel"),
                        ButtonKind::Secondary,
                        |ui: &mut Ui, _| ui.files.armed = None,
                    )),
            ),
    )
}

fn delete(ui: &mut Ui, armed: Armed) {
    ui.files.armed = None;
    match armed {
        Armed::Version(id, version) => {
            change(ui, &id, |record| {
                record.versions.retain(|v| v.id != version)
            });
            ui.files.tab = None;
        }
        Armed::File(id) => {
            if !ui.files.fixture {
                worker::cancel_record(&id);
                if let Err(error) = store::delete(&id) {
                    ui.files.error = Some(format!("{error:#}"));
                    return;
                }
            }
            ui.files.records.retain(|listed| listed.record.id != id);
            ui.files.selected = None;
            ui.files.stale = true;
        }
    }
}

/// Timestamps | Plain text, smaller than the tabs above it, the first with
/// a clock.
fn timestamp_switch(p: Paint, timed: bool) -> El {
    let c = &p.k.c;
    let segment = |key: &str, label: &str, glyph: Option<&'static rune::Icon>, on: bool| {
        let mut cell = row()
            .key(key)
            .role("button")
            .label(label)
            .focusable(true)
            .h(20.0)
            .px(7.0)
            .gap(4.0)
            .center_y()
            .radius(p.k.radius.sm);
        cell = if on {
            cell.fill(c.layer_02).box_shadow(rune::Shadow {
                spread: 0.5,
                color: c.border_strong,
                ..Default::default()
            })
        } else {
            cell.hover(c.hover)
        };
        let ink = if on { p.text() } else { p.muted() };
        if let Some(glyph) = glyph {
            cell = cell.child(icon(glyph, 11.0).key("icon").color(ink));
        }
        let timed_now = key == "timed";
        cell.child(
            text(label.to_string())
                .key("words")
                .font(p.k.ty.small_medium)
                .color(ink),
        )
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| {
            tx.settle();
            ui.files.timestamps = Some(timed_now);
        })
    };
    row()
        .key("file-timestamps")
        .role("segmented")
        .gap(2.0)
        .px(2.0)
        .py(2.0)
        .center_y()
        .radius(p.k.radius.md)
        .fill(c.layer_01)
        .box_shadow(rune::Shadow {
            spread: 0.5,
            color: c.border_base,
            ..Default::default()
        })
        .child(segment("timed", tr("Timestamps"), Some(&CLOCK_ICON), timed))
        .child(segment("plain", tr("Plain text"), None, !timed))
}
