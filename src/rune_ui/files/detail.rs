//! A file's card: its tabs (the versions of its text and the jobs that will
//! add more), the open version with its line, actions, and text, and the
//! request field under it.

use rune::{El, TextEdit, Tx, col, icon, row, text};
use rune_kit::{
    IconButton, IconButtonVariant, Menu as KitMenu, MenuItem, Placement, TextArea, TextInput,
};

use super::super::controls::{ButtonKind, DESCRIPTION, Paint, button, segmented_tx};
use super::super::list_detail as ld;
use super::super::{Menu, Pane, Ui};
use super::tabs;
use super::{Again, Armed, Tab, failure, stage_label, submit};
use crate::desktop::i18n::{tr, tr_fill, tr_named};
use crate::files::store::{self, FileRecord, Version, VersionKind};
use crate::files::text as file_text;
use crate::files::worker::{self, Job, Stage, Task};

static MORE_ICON: rune::Icon = rune::Icon {
    name: "hex-files-more",
    view: 16.0,
    body: r#"<circle cx="3.5" cy="8" r="1.25" fill="currentColor"/><circle cx="8" cy="8" r="1.25" fill="currentColor"/><circle cx="12.5" cy="8" r="1.25" fill="currentColor"/>"#,
};

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

static SEND_ICON: rune::Icon = rune::Icon {
    name: "hex-files-send",
    view: 16.0,
    body: r#"<path d="M8 13V3.5M3.8 7.5 8 3.3l4.2 4.2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
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
fn change(ui: &mut Ui, id: &str, change: impl Fn(&mut FileRecord)) {
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
fn tab_label(name: &str) -> String {
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
fn texts(ui: &Ui, version: &Version) -> (String, String) {
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
        actions.push(copy_menu(ui, p, version, layer));
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
        actions.push(more_menu(ui, p, record, version, layer));
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
                body = body.child(progress(p, record, task));
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
                body = body.child(again_row(ui, p, again, layer));
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
            } else {
                body = body.child(content(ui, p, version));
                ask = Some(ask_area(ui, p, version, layer));
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
    col()
        .key("file-detail")
        .grow(1.0)
        .shrink()
        .child(header)
        .child(ld::card_scroll("file-text", body))
        .children(ask)
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
                "{model} in a {window}K window",
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
    let mut line = row().key("version-line").gap(8.0).center_y();
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
        line = line.child(segmented_tx(
            p,
            "file-timestamps",
            &[tr("Timestamps"), tr("Plain text")],
            selected,
            |ui: &mut Ui, index: usize, tx: &mut Tx| {
                tx.settle();
                ui.files.timestamps = Some(index == 0);
            },
        ));
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
            line = line.child(again_menu(ui, p, record, layer));
        }
        VersionKind::Cleanup => line = line.child(redo_menu(ui, p, record, version, layer)),
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
                "The text is longer than the model's window, so it went in 3 steps over {} parts. A model with a large window reads it whole: OpenRouter or OpenCode.",
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
                .font(p.k.ty.body)
                .color(p.text())
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
                            .font(p.k.ty.mono)
                            .color(p.faint())
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
    let mut card = col()
        .key("file-progress")
        .gap(10.0)
        .px(14.0)
        .py(12.0)
        .radius(p.k.radius.md)
        .border(p.hair, p.k.c.border_weak)
        .child(
            row()
                .key("head")
                .gap(10.0)
                .center_y()
                .child(
                    text(title)
                        .key("title")
                        .font(p.k.ty.ui_medium)
                        .color(p.text())
                        .grow(1.0)
                        .shrink()
                        .truncate(),
                )
                .children((task.stage != Stage::Queued).then(|| {
                    text(format!("{percent}%"))
                        .key("percent")
                        .font(p.k.ty.small)
                        .color(p.muted())
                }))
                .child(button(
                    p,
                    "file-stop",
                    if task.stage == Stage::Queued {
                        tr("Remove")
                    } else {
                        tr("Stop")
                    },
                    ButtonKind::Secondary,
                    move |ui: &mut Ui, _| {
                        if !ui.files.fixture {
                            worker::cancel(id);
                        }
                        ui.files.stale = true;
                    },
                )),
        )
        .child(
            rune_kit::Progress::new(Some(task.fraction.clamp(0.0, 1.0)))
                .build(p.k)
                .key("bar"),
        );
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

/// The three steps, the current one in full ink.
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
    let mut list = col().key("steps").gap(4.0).child(
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
        let (mark, ink) = match index.cmp(&step) {
            std::cmp::Ordering::Less => ("✓", p.muted()),
            std::cmp::Ordering::Equal => ("•", p.text()),
            std::cmp::Ordering::Greater => ("○", p.faint()),
        };
        list = list.child(
            row()
                .key(format!("step{index}"))
                .gap(8.0)
                .child(text(mark).key("mark").font(p.k.ty.small).color(ink).w(12.0))
                .child(
                    text(label)
                        .key("label")
                        .font(p.k.ty.small)
                        .color(ink)
                        .wrap(),
                ),
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

fn menu_open(ui: &Ui, key: &'static str) -> bool {
    ui.open == Some(Menu::Pick(key))
}

fn toggle(ui: &mut Ui, key: &'static str) {
    ui.open = (ui.open != Some(Menu::Pick(key))).then_some(Menu::Pick(key));
}

/// Copy as Markdown or as plain text.
fn copy_menu(ui: &Ui, p: Paint, version: &Version, layer: &mut Vec<El>) -> El {
    let (markdown, plain) = texts(ui, version);
    let copied = |what| ui.files.copied == Some((version.id, what));
    let id = version.id;
    let item = |label: &str, what: &'static str, copy: String| {
        MenuItem::new(if copied(what) { tr("Copied") } else { label }).on_select_tx(
            move |ui: &mut Ui, tx: &mut Tx| {
                tx.copy(copy.clone());
                ui.files.copied = Some((id, what));
                ui.open = Some(Menu::Pick("files-copy"));
            },
        )
    };
    let trigger = IconButton::new(ld::COPY_ICON)
        .variant(IconButtonVariant::Ghost)
        .control_size(rune_kit::ControlSize::Small)
        .label(tr("Copy"))
        .on_press(|ui: &mut Ui| toggle(ui, "files-copy"))
        .build(p.k)
        .key("file-copy");
    KitMenu::new("files-copy-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(200.0)
        .open(menu_open(ui, "files-copy"))
        .on_close(|ui: &mut Ui| ui.open = None)
        .item(item(tr("Copy as Markdown"), "markdown", markdown))
        .item(item(tr("Copy as plain text"), "plain", plain))
        .trigger(trigger)
        .build(p.k, layer)
}

/// Where the file is: `In Downloads`, its folder's name.
fn short_path(path: &std::path::Path) -> String {
    let folder = path
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|folder| folder.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".into());
    tr_fill("In {}", &folder)
}

/// Save as…, Show in Finder, Open original, the two deletes, and where the
/// file is.
fn more_menu(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    version: Option<&Version>,
    layer: &mut Vec<El>,
) -> El {
    let mut menu = KitMenu::new("files-more-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(220.0)
        .open(menu_open(ui, "files-more"))
        .on_close(|ui: &mut Ui| ui.open = None);
    if let Some(version) = version {
        let (markdown, plain) = texts(ui, version);
        let stem = record
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| record.name.clone());
        let file_name = format!("{stem} — {}.md", tab_label(&version.name));
        let directory = record.path.parent().map(std::path::Path::to_path_buf);
        menu = menu.item(MenuItem::new(tr("Save as…")).on_select_tx(
            move |ui: &mut Ui, tx: &mut Tx| {
                ui.open = None;
                let options = rune::FileDialogOptions {
                    title: tr("Save the text").into(),
                    directory: directory.clone(),
                    file_name: Some(file_name.clone()),
                    ..Default::default()
                }
                .filter("Markdown", &["md"])
                .filter(tr("Plain text"), &["txt"]);
                let (markdown, plain) = (markdown.clone(), plain.clone());
                tx.save_file(options, move |ui: &mut Ui, chosen, _| match chosen {
                    Ok(Some(path)) => {
                        let plain_text = path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));
                        let body = if plain_text { &plain } else { &markdown };
                        if let Err(error) = std::fs::write(&path, body) {
                            ui.files.error = Some(error.to_string());
                        }
                    }
                    Ok(None) => {}
                    Err(error) => ui.files.error = Some(error.to_string()),
                });
            },
        ));
    }
    let found = ui.files.fixture || record.path.exists();
    let path = record.path.clone();
    let shown = path.clone();
    menu = menu
        .item(
            MenuItem::new(tr("Show in Finder"))
                .disabled(!found)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    if !ui.files.fixture {
                        super::reveal(&path);
                    }
                }),
        )
        .item(
            MenuItem::new(tr("Open original"))
                .disabled(!found)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    if !ui.files.fixture {
                        super::open_file(&shown);
                    }
                }),
        );
    menu = menu.separator();
    if let Some(version) = version
        && record.versions.first().map(|first| first.id) != Some(version.id)
    {
        let armed = Armed::Version(record.id.clone(), version.id);
        menu = menu.item(MenuItem::new(tr("Delete this version…")).on_select(
            move |ui: &mut Ui| {
                ui.open = None;
                ui.files.armed = Some(armed.clone());
            },
        ));
    }
    let armed = Armed::File(record.id.clone());
    menu = menu
        .item(
            MenuItem::new(tr("Delete file from list…")).on_select(move |ui: &mut Ui| {
                ui.open = None;
                ui.files.armed = Some(armed.clone());
            }),
        )
        .separator()
        .group_label(if found {
            short_path(&record.path)
        } else {
            tr("File not found").into()
        });
    let trigger = IconButton::new(&MORE_ICON)
        .variant(IconButtonVariant::Ghost)
        .control_size(rune_kit::ControlSize::Small)
        .label(tr("More"))
        .on_press(|ui: &mut Ui| toggle(ui, "files-more"))
        .build(p.k)
        .key("file-more");
    menu.trigger(trigger).build(p.k, layer)
}

/// Transcribe again with another downloaded model, as a new Original tab.
fn again_menu(ui: &Ui, p: Paint, record: &FileRecord, layer: &mut Vec<El>) -> El {
    let gone = !ui.files.fixture && !record.path.is_file();
    let mut menu = KitMenu::new("files-again-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(220.0)
        .open(!gone && menu_open(ui, "files-again"))
        .on_close(|ui: &mut Ui| ui.open = None)
        .group_label(tr("Downloaded models"));
    for &(model, name) in &ui.files.models {
        let id = record.id.clone();
        menu = menu.item(
            MenuItem::new(name).on_select(move |ui: &mut Ui| choose_model(ui, &id, model, name)),
        );
    }
    let trigger = rune_kit::Button::new(if gone {
        tr("The file is gone")
    } else {
        tr("Transcribe again")
    })
    .variant(rune_kit::ButtonVariant::Neutral)
    .control_size(rune_kit::ControlSize::Small)
    .disabled(gone)
    .on_press(|ui: &mut Ui| {
        ui.files.models = super::super::history::installed_models(ui);
        toggle(ui, "files-again");
    })
    .build(p.k)
    .key("file-again");
    menu.trigger(trigger).build(p.k, layer)
}

/// A model chosen from Transcribe again: one that takes a language asks
/// for it first, starting at the saved language; any other runs at once.
fn choose_model(
    ui: &mut Ui,
    record: &str,
    model: crate::transcription::models::TranscriptionModelId,
    name: &'static str,
) {
    ui.open = None;
    let Some(choices) = super::super::history::language_choices(model) else {
        ui.files.again = None;
        transcribe_again(ui, record, model, name, None);
        return;
    };
    let saved = &ui.settings.transcription.language;
    let language = if choices.contains(&saved.as_str()) {
        saved.clone()
    } else {
        ui.settings.transcription_for_model(model).language
    };
    ui.files.again = Some(Again {
        record: record.into(),
        model,
        name,
        language,
    });
}

/// Queues Transcribe again with `model`, in `language` when given.
fn transcribe_again(
    ui: &mut Ui,
    record: &str,
    model: crate::transcription::models::TranscriptionModelId,
    name: &'static str,
    language: Option<String>,
) {
    let mut selection = ui.settings.transcription_for_model(model);
    if let Some(language) = language {
        selection.language = language;
    }
    let job = Job::Transcribe {
        selection: Some(selection),
        model_name: name.into(),
        again: true,
    };
    if let Some(task) = submit(ui, record, job) {
        ui.files.tab = Some(Tab::Task(task));
    }
}

/// The language for Transcribe again's model, with Transcribe and Cancel.
fn again_row(ui: &Ui, p: Paint, again: &Again, layer: &mut Vec<El>) -> El {
    use crate::transcription::models::language_name;
    let choices = super::super::history::language_choices(again.model).unwrap_or_default();
    let labels: Vec<&str> = choices.iter().map(|code| language_name(code)).collect();
    let languages = rune_kit::Select::new("files-again-language", &labels)
        .appearance(rune_kit::SelectAppearance::Inline)
        .current(Some(language_name(&again.language)))
        .open(menu_open(ui, "files-again-language"))
        .on_toggle(|ui: &mut Ui| toggle(ui, "files-again-language"))
        .on_close(|ui: &mut Ui| ui.open = None)
        .on_choose(|ui: &mut Ui, choice: &str, _: &mut Tx| {
            ui.open = None;
            if let Some(again) = &mut ui.files.again
                && let Some(code) =
                    super::super::history::language_choices(again.model).and_then(|choices| {
                        choices
                            .into_iter()
                            .find(|code| language_name(code) == choice)
                    })
            {
                again.language = code.into();
            }
        })
        .build(p.k, layer);
    row()
        .key("again-pending")
        .gap(8.0)
        .center_y()
        .child(
            text(again.name)
                .key("model")
                .font(DESCRIPTION)
                .color(p.muted())
                .shrink()
                .truncate(),
        )
        .child(languages)
        .child(rune::el().key("spacer").grow(1.0))
        .child(
            rune_kit::Button::new(tr("Transcribe"))
                .variant(rune_kit::ButtonVariant::Neutral)
                .control_size(rune_kit::ControlSize::Small)
                .on_press(|ui: &mut Ui| {
                    if let Some(again) = ui.files.again.take() {
                        transcribe_again(
                            ui,
                            &again.record,
                            again.model,
                            again.name,
                            Some(again.language),
                        );
                    }
                })
                .build(p.k)
                .key("again-start"),
        )
        .child(
            rune_kit::Button::new(tr("Cancel"))
                .variant(rune_kit::ButtonVariant::Ghost)
                .control_size(rune_kit::ControlSize::Small)
                .on_press(|ui: &mut Ui| ui.files.again = None)
                .build(p.k)
                .key("again-cancel"),
        )
}

/// Clean up again on another engine, as a new Cleanup version.
fn redo_menu(ui: &Ui, p: Paint, record: &FileRecord, version: &Version, layer: &mut Vec<El>) -> El {
    let from = version
        .from
        .or(record.versions.first().map(|first| first.id))
        .unwrap_or(version.id);
    let mut menu = KitMenu::new("files-redo-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(240.0)
        .open(menu_open(ui, "files-redo"))
        .on_close(|ui: &mut Ui| ui.open = None);
    let engines = &ui.files.engines;
    for (local, label) in [
        (true, tr("Built in, downloaded")),
        (false, tr("Set up in Output › AI cleanup")),
    ] {
        let group: Vec<_> = engines.iter().filter(|e| e.local == local).collect();
        if group.is_empty() {
            continue;
        }
        menu = menu.group_label(label);
        for engine in group {
            let (id, value) = (record.id.clone(), engine.value.clone());
            menu = menu.item(
                MenuItem::new(engine.label.clone()).on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    let job = Job::Cleanup {
                        from,
                        engine: Some(value.clone()),
                    };
                    if let Some(task) = submit(ui, &id, job) {
                        ui.files.tab = Some(Tab::Task(task));
                    }
                }),
            );
        }
    }
    menu = menu.separator().item(
        MenuItem::new(tr("More models in Output › AI cleanup")).on_select(|ui: &mut Ui| {
            ui.open = None;
            ui.pane = Pane::Output;
        }),
    );
    let trigger = rune_kit::Button::new(tr("Clean up again"))
        .variant(rune_kit::ButtonVariant::Neutral)
        .control_size(rune_kit::ControlSize::Small)
        .on_press(|ui: &mut Ui| {
            ui.files.engines = super::settings::engine_choices(ui);
            toggle(ui, "files-redo");
        })
        .build(p.k)
        .key("file-redo");
    menu.trigger(trigger).build(p.k, layer)
}

/// Sends `prompt` to work on the open version, as a new tab.
fn send(ui: &mut Ui, prompt: String) {
    let prompt = prompt.trim().to_string();
    let Some(record) = ui.files.selected() else {
        return;
    };
    let Some(Tab::Version(from)) = current(ui, record) else {
        return;
    };
    if prompt.is_empty() {
        return;
    }
    let quick = &ui.settings.files.quick_prompts;
    let name = record.unique_name(&worker::request_name(&prompt, quick));
    let typed = !quick.iter().any(|chip| chip.prompt.trim() == prompt);
    let id = record.id.clone();
    if typed && !ui.files.fixture {
        if let Err(error) = store::remember(&prompt) {
            tracing::warn!(%error, "couldn't remember the request");
        }
        ui.files.recent = store::recent();
    }
    if let Some(task) = submit(ui, &id, Job::Request { from, prompt, name }) {
        ui.files.tab = Some(Tab::Task(task));
    }
    ui.files.ask = TextEdit::default();
}

/// The quick requests, Recent, and the request field.
fn ask_area(ui: &Ui, p: Paint, version: &Version, layer: &mut Vec<El>) -> El {
    let mut chips = row().key("chips").gap(6.0).center_y();
    for (index, chip) in ui.settings.files.quick_prompts.iter().enumerate() {
        let prompt = chip.prompt.clone();
        chips = chips.child(
            rune_kit::Button::new(chip.label.clone())
                .variant(rune_kit::ButtonVariant::Neutral)
                .control_size(rune_kit::ControlSize::Small)
                .on_press(move |ui: &mut Ui| send(ui, prompt.clone()))
                .build(p.k)
                .key(format!("chip{index}")),
        );
    }
    if !ui.files.recent.is_empty() {
        let mut menu = KitMenu::new("files-recent-menu")
            .placement(Placement::TopStart)
            .gutter(4.0)
            .min_width(280.0)
            .open(menu_open(ui, "files-recent"))
            .on_close(|ui: &mut Ui| ui.open = None)
            .group_label(tr("Recent requests"));
        for prompt in &ui.files.recent {
            let chosen = prompt.clone();
            menu = menu.item(MenuItem::new(prompt.clone()).on_select(move |ui: &mut Ui| {
                ui.open = None;
                ui.files.ask = TextEdit::new(chosen.clone());
            }));
        }
        if !ui.files.fixture {
            menu = menu
                .separator()
                .item(MenuItem::new(tr("Clear recent")).on_select(|ui: &mut Ui| {
                    ui.open = None;
                    if let Err(error) = store::clear_recent() {
                        tracing::warn!(%error, "couldn't clear recent requests");
                    }
                    ui.files.recent.clear();
                }));
        }
        let trigger = rune_kit::Button::new(tr("Recent"))
            .variant(rune_kit::ButtonVariant::Ghost)
            .control_size(rune_kit::ControlSize::Small)
            .icon(p.k.glyph.chevron_down)
            .on_press(|ui: &mut Ui| toggle(ui, "files-recent"))
            .build(p.k)
            .key("recent");
        chips = chips.child(menu.trigger(trigger).build(p.k, layer));
    }
    let send_button = IconButton::new(&SEND_ICON)
        .variant(IconButtonVariant::Neutral)
        .control_size(rune_kit::ControlSize::Small)
        .label(tr("Send"))
        .on_press(|ui: &mut Ui| {
            let prompt = ui.files.ask.text.clone();
            send(ui, prompt);
        })
        .build(p.k)
        .key("send");
    let field = TextInput::new(&ui.files.ask)
        .placeholder(tr("What should I do with this text?"))
        .width(None)
        .trailing(send_button)
        .on_edit(|ui: &mut Ui| &mut ui.files.ask)
        .on_submit(|ui: &mut Ui, _| {
            let prompt = ui.files.ask.text.clone();
            send(ui, prompt);
        })
        .build(p.k)
        .key("files-ask");
    col()
        .key("files-ask-area")
        .gap(8.0)
        .pt(12.0)
        .pb(4.0)
        .child(chips)
        .child(field)
        .child(
            text(tr_fill(
                "Works on {} and adds the result to Requests.",
                &tabs::title(ui, version),
            ))
            .key("hint")
            .font(p.k.ty.small)
            .color(p.faint()),
        )
}
