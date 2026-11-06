//! The Files pane (Labs › Files, macOS): audio and video files dropped or
//! chosen, transcribed by a queue, cleaned up, and reworked with requests,
//! each result a tab of the file's text. The header's `Files | Settings`
//! switch shows the feature's settings.
//!
//! The records live in `files::store` and the queue in `files::worker`; the
//! pane reads them again whenever the worker reports a change. Scenes feed
//! [`FilesPane::fixture`], which never touches the store.

use std::path::PathBuf;

use rune::{El, TextEdit, Tx, col, row, text};
use rune_kit::TextInput;

use super::controls::{ButtonKind, DESCRIPTION, Paint, button, segmented_tx};
use super::list_detail::{self as ld, Item};
use super::lists::{dismissible_error_panel, error_panel};
use super::{Pane, Ui};
use crate::desktop::i18n::{tr, tr_fill};
use crate::files::store::{self, FileRecord};
use crate::files::worker::{self, Config, Job, Stage, Task};
use crate::files::{decode, text as file_text};
use crate::transcription::models::TranscriptionModelId;

mod actions;
mod detail;
pub(super) mod settings;
mod tabs;

/// What the pane shows, picked in its header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum View {
    #[default]
    Files,
    Settings,
}

/// What a two-step delete has armed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Armed {
    Version(String, u32),
    File(String),
}

/// The open tab: a version, or a job that will add one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    Version(u32),
    Task(u64),
}

/// Transcribe again's model while its language is being chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Again {
    pub(super) record: String,
    pub(super) model: TranscriptionModelId,
    pub(super) name: &'static str,
    pub(super) language: String,
}

/// A record with the day and time the list shows it under.
#[derive(Clone, Debug)]
pub(super) struct Listed {
    pub(super) record: FileRecord,
    /// `Today`, `Yesterday`, or a date.
    pub(super) day: String,
    pub(super) clock: String,
}

#[derive(Default)]
pub(super) struct FilesPane {
    pub(super) view: View,
    /// Newest first.
    pub(super) records: Vec<Listed>,
    pub(super) tasks: Vec<Task>,
    /// The queue as last read without its progress, so progress alone
    /// doesn't read the store again.
    seen: Option<Vec<(u64, String, Stage)>>,
    /// Read on the next poll.
    stale: bool,
    pub(super) recent: Vec<String>,
    pub(super) search: TextEdit,
    pub(super) selected: Option<String>,
    pub(super) tab: Option<Tab>,
    /// The version last opened in each of a file's tabs, until Hex quits.
    last: std::collections::HashMap<(String, tabs::Group), Tab>,
    /// Original tabs' view until the setting changes; `None` follows it.
    pub(super) timestamps: Option<bool>,
    pub(super) editing: Option<u32>,
    pub(super) edit: TextEdit,
    pub(super) ask: TextEdit,
    drop_over: bool,
    drop_rejected: bool,
    pub(super) armed: Option<Armed>,
    /// What was last copied, for the menu's "Copied".
    pub(super) copied: Option<(u32, &'static str)>,
    pub(super) error: Option<String>,
    /// Transcribe again's model waiting for its language.
    pub(super) again: Option<Again>,
    /// Transcribe again's models, read when its menu opens.
    pub(super) models: Vec<(TranscriptionModelId, &'static str)>,
    /// The app that opens the file, read when the … menu opens.
    pub(super) opener: Option<String>,
    /// Clean up again's engines, read when its menu opens.
    pub(super) engines: Vec<settings::EngineChoice>,
    /// Settings' own prompt while it holds typing not yet saved.
    pub(super) prompt: Option<TextEdit>,
    /// The quick request being written in Settings.
    pub(super) quick: Option<settings::QuickEdit>,
    /// The worker wakes this window.
    woken: bool,
    /// A scene: nothing reads or writes the store.
    fixture: bool,
}

/// The scenes' "now": Thursday, November 5, 2026, 15:10 UTC.
const FIXTURE_NOW_MS: u64 = 1_793_891_400_000;

/// Labs › Files is on, on a Mac.
pub(super) fn available(ui: &Ui) -> bool {
    ui.platform.caps.os == super::platform::Os::Macos && ui.settings.labs.files
}

impl FilesPane {
    pub(super) fn record(&self, id: &str) -> Option<&FileRecord> {
        self.records
            .iter()
            .map(|listed| &listed.record)
            .find(|record| record.id == id)
    }

    /// Now, or the scenes' fixed moment.
    pub(super) fn now(&self) -> u64 {
        if self.fixture {
            FIXTURE_NOW_MS
        } else {
            store::now_ms()
        }
    }

    pub(super) fn selected(&self) -> Option<&FileRecord> {
        self.selected.as_deref().and_then(|id| self.record(id))
    }

    /// Jobs on `record`, the running one first.
    pub(super) fn tasks_of<'a>(&'a self, record: &'a str) -> impl Iterator<Item = &'a Task> {
        self.tasks.iter().filter(move |task| task.record == record)
    }

    pub(super) fn select(&mut self, id: String) {
        if self.selected.as_deref() != Some(id.as_str()) {
            self.tab = None;
            self.editing = None;
            self.armed = None;
            self.copied = None;
            self.again = None;
        }
        self.selected = Some(id);
    }

    fn visible(&self) -> Vec<&Listed> {
        let query = self.search.text.trim().to_lowercase();
        self.records
            .iter()
            .filter(|listed| {
                query.is_empty()
                    || listed.record.name.to_lowercase().contains(&query)
                    || listed
                        .record
                        .versions
                        .iter()
                        .any(|version| version.text.to_lowercase().contains(&query))
            })
            .collect()
    }

    /// Reads the records, the queue, and recent requests again.
    fn reload(&mut self) {
        let now = store::now_ms();
        self.records = store::list()
            .into_iter()
            .map(|record| Listed {
                day: crate::meeting::day_label(record.added_ms, now),
                clock: crate::meeting::clock(record.added_ms),
                record,
            })
            .collect();
        self.recent = store::recent();
        if let Some(selected) = &self.selected
            && self.record(selected).is_none()
        {
            self.selected = None;
            self.tab = None;
        }
    }

    /// A finished interview with three tabs, a podcast transcribing, a
    /// lecture waiting, and a recording without audio.
    pub(super) fn fixture() -> Self {
        use crate::files::store::{Segment, Version, VersionKind};
        let ago = |minutes: u64| FIXTURE_NOW_MS - minutes * 60_000;
        let record = |id: &str, name: &str, minutes: u64| FileRecord {
            id: id.into(),
            name: name.into(),
            path: PathBuf::from(format!("/Users/me/Downloads/{name}")),
            added_ms: 0,
            duration_ms: minutes * 60_000 + 10_000,
            versions: Vec::new(),
            error: None,
        };
        let segment = |start: u64, text: &str| Segment {
            start_ms: start * 1_000,
            end_ms: start * 1_000 + 9_000,
            text: text.into(),
        };
        let segments = vec![
            segment(
                0,
                "We started with the client building their reports by hand for the third month: they export from the CRM, merge it in Excel, and send it to their manager once a week.",
            ),
            segment(
                21,
                "The main pain is that it takes almost a whole day. Mistakes only show up on the call, and then everything is rebuilt.",
            ),
            segment(
                48,
                "We discussed connecting the CRM export directly and building the report every Friday. The client will try it with one team if we show a prototype by the end of the month.",
            ),
        ];
        let original = Version {
            id: 1,
            kind: VersionKind::Original,
            name: "Original".into(),
            text: file_text::plain(&segments),
            segments,
            model: "Parakeet v3".into(),
            created_ms: ago(50),
            ..Version::default()
        };
        let cleanup = Version {
            id: 2,
            kind: VersionKind::Cleanup,
            name: "Cleanup".into(),
            text: "We started with the client building reports by hand for the third month: they export data from the CRM, merge it in Excel, and send it to their manager weekly.\n\nThe main pain: it takes almost a whole day. Mistakes only surface on the call, and then everything has to be rebuilt.\n\nWe discussed connecting the CRM export directly and building the report automatically every Friday. The client is ready to try it with one team if we show a prototype by the end of the month.".into(),
            model: "Qwen3 4B".into(),
            from: Some(1),
            window_k: Some(8),
            created_ms: ago(48),
            ..Version::default()
        };
        let summary = Version {
            id: 3,
            kind: VersionKind::Request,
            name: "Summary".into(),
            text: "## Summary\n\n- Reports are built **by hand**: CRM → Excel, about a day a week\n- Mistakes only surface on the call\n- Next: an automatic Friday report, piloted with one team\n- Prototype due by the end of the month".into(),
            model: "Qwen3 4B".into(),
            from: Some(2),
            prompt: None,
            window_k: Some(32),
            parts: Some(6),
            created_ms: ago(30),
            ..Version::default()
        };
        let again = Version {
            id: 4,
            name: "Original (Whisper Large v3 Turbo)".into(),
            model: "Whisper Large v3 Turbo".into(),
            created_ms: ago(26 * 60),
            ..original.clone()
        };
        let gemma = Version {
            id: 5,
            name: "Cleanup (Gemma 4 E4B)".into(),
            model: "Gemma 4 E4B".into(),
            created_ms: ago(24 * 60),
            ..cleanup.clone()
        };
        let actions = Version {
            id: 6,
            name: "Action items".into(),
            text: "- [ ] Connect the CRM export\n- [ ] Show a prototype by the end of the month"
                .into(),
            prompt: None,
            window_k: Some(8),
            parts: None,
            created_ms: ago(20),
            ..summary.clone()
        };
        let letter = Version {
            id: 7,
            name: "Rewrite as an email".into(),
            text: "Hi Anna,\n\nThanks for the call. Next we connect the CRM export and show a prototype by the end of the month.".into(),
            prompt: Some("Rewrite it as an email to the client: short, in points, without jargon, with the outcome of the meeting and a date for each step".into()),
            model: "Claude Sonnet 4.6".into(),
            window_k: None,
            parts: None,
            created_ms: ago(12),
            ..summary.clone()
        };
        let mut interview = record(
            "preview-interview",
            "Client interview about automating CRM and Excel reports, part 2.m4a",
            42,
        );
        interview.versions = vec![original, cleanup, summary, again, gemma, actions, letter];
        let mut planning = record("preview-planning", "Planning, November 4.mov", 27);
        planning.versions = interview.versions[..2].to_vec();
        let mut zoom = record(
            "preview-zoom",
            "Zoom_Meeting_2026-11-04_18-05-33_recording_final_v2_audio_only.m4a",
            31,
        );
        zoom.versions = interview.versions[..2].to_vec();
        let podcast = record("preview-podcast", "Podcast #41.mp3", 54);
        let lecture = record("preview-lecture", "Lecture 3.mp4", 72);
        let mut silent = record("preview-silent", "Screen recording.mov", 0);
        silent.error = Some("This file has no audio.".into());
        let listed = |record: FileRecord, day: &str, clock: &str| Listed {
            record,
            day: day.into(),
            clock: clock.into(),
        };
        let task = |id, record: &str, stage, fraction| Task {
            id,
            record: record.into(),
            stage,
            fraction,
            parts: None,
            name: None,
            kind: VersionKind::Original,
            from: None,
            model: None,
            window_k: None,
            draft: None,
        };
        Self {
            records: vec![
                listed(lecture, "Today", "15:02"),
                listed(podcast, "Today", "14:58"),
                listed(interview, "Today", "14:20"),
                listed(silent, "Today", "13:02"),
                listed(planning, "Yesterday", "18:05"),
                listed(zoom, "Yesterday", "11:40"),
            ],
            tasks: vec![
                task(1, "preview-podcast", Stage::Transcribing, 0.58),
                task(2, "preview-lecture", Stage::Queued, 0.0),
            ],
            recent: vec![
                "Turn this into a post for the team chat".into(),
                "Translate to English".into(),
            ],
            selected: Some("preview-interview".into()),
            tab: Some(Tab::Version(2)),
            fixture: true,
            ..Self::default()
        }
    }

    /// No files yet.
    pub(super) fn empty_fixture() -> Self {
        Self {
            fixture: true,
            ..Self::default()
        }
    }

    /// The fixture with a request working in three steps.
    pub(super) fn working_fixture() -> Self {
        let mut pane = Self::fixture();
        pane.tasks.insert(
            0,
            Task {
                id: 3,
                record: "preview-interview".into(),
                stage: Stage::Working,
                fraction: 0.45,
                parts: Some(6),
                name: Some("Structure".into()),
                kind: crate::files::store::VersionKind::Request,
                from: Some(2),
                model: Some("Qwen3 4B".into()),
                window_k: Some(32),
                draft: Some(crate::files::worker::Draft {
                    kind: crate::files::rewrite::DraftKind::Outline,
                    text: "1. Как клиент собирает отчёты сейчас\n2. Сколько это стоит команде\n\
                           3. Автоматическая выгрузка из CRM"
                        .into(),
                    cut: false,
                }),
            },
        );
        pane.tab = Some(Tab::Task(3));
        pane
    }
}

/// Follows the worker: the queue on every change, the records when a job
/// starts, moves on, or ends.
pub(super) fn poll(ui: &mut Ui) -> bool {
    if ui.pane != Pane::Files || !available(ui) || ui.files.fixture || ui.is_fixture() {
        return false;
    }
    if !ui.files.woken
        && let Some(waker) = ui.modes.waker.clone()
    {
        worker::set_waker(move || waker());
        ui.files.woken = true;
    }
    let tasks = worker::tasks();
    let shape: Vec<(u64, String, Stage)> = tasks
        .iter()
        .map(|task| (task.id, task.record.clone(), task.stage))
        .collect();
    let mut changed = tasks != ui.files.tasks;
    ui.files.tasks = tasks;
    if ui.files.stale || ui.files.seen.as_ref() != Some(&shape) {
        ui.files.seen = Some(shape);
        ui.files.stale = false;
        ui.files.reload();
        changed = true;
    }
    if let Some(Tab::Task(id)) = ui.files.tab
        && !ui.files.tasks.iter().any(|task| task.id == id)
    {
        // The job finished: open what it made, the record's newest tab.
        ui.files.tab = ui
            .files
            .selected()
            .and_then(|record| record.versions.last())
            .map(|version| Tab::Version(version.id));
        changed = true;
    }
    changed
}

/// Queues `job` on `record` with the settings as they are now.
pub(super) fn submit(ui: &mut Ui, record: &str, job: Job) -> Option<u64> {
    if ui.files.fixture {
        return None;
    }
    let config = Config::from_settings(&ui.drafted());
    let id = worker::submit(record, job, config, false);
    ui.files.stale = true;
    (id != 0).then_some(id)
}

/// Adds the supported files of `paths` and queues their transcription,
/// selecting the first.
pub(super) fn add(ui: &mut Ui, paths: &[PathBuf]) {
    let mut first = None;
    for path in paths.iter().filter(|path| decode::supported(path)) {
        let record = FileRecord::new(path, store::now_ms());
        let id = record.id.clone();
        if ui.files.fixture {
            ui.files.records.insert(
                0,
                Listed {
                    record,
                    day: "Today".into(),
                    clock: "15:10".into(),
                },
            );
        } else {
            if let Err(error) = store::save(&record) {
                ui.files.error = Some(format!("{error:#}"));
                continue;
            }
            let config = Config::from_settings(&ui.drafted());
            let job = Job::Transcribe {
                selection: ui.settings.files.selection(&ui.settings.transcription),
                model_name: config.transcription_name.clone(),
                again: false,
            };
            worker::submit(&id, job, config, false);
        }
        first.get_or_insert(id);
    }
    ui.files.stale = true;
    if let Some(id) = first {
        ui.files.select(id);
    }
}

/// Opens the file dialog in the last folder a file was chosen from.
fn choose(ui: &mut Ui, tx: &mut Tx) {
    let options = rune::FileDialogOptions {
        title: tr("Choose an audio or video file").into(),
        directory: ui.settings.files.last_folder.clone().map(PathBuf::from),
        ..Default::default()
    }
    .filter(tr("Audio and video"), decode::EXTENSIONS);
    tx.open_file(options, |ui: &mut Ui, chosen, _| match chosen {
        Ok(Some(path)) => {
            if let Some(folder) = path.parent().map(|folder| folder.display().to_string())
                && ui.settings.files.last_folder.as_deref() != Some(folder.as_str())
            {
                ui.update(|settings| settings.files.last_folder = Some(folder));
            }
            add(ui, &[path]);
        }
        Ok(None) => {}
        Err(error) => ui.files.error = Some(error.to_string()),
    });
}

/// The header's `Files | Settings` switch; none while Files is off.
pub(super) fn view_switch(ui: &Ui, p: Paint) -> Option<El> {
    if !available(ui) {
        return None;
    }
    let views = [tr("Files"), tr("Settings")];
    let selected = match ui.files.view {
        View::Files => 0,
        View::Settings => 1,
    };
    Some(segmented_tx(
        p,
        "files-view",
        &views,
        selected,
        |ui: &mut Ui, index: usize, tx: &mut Tx| {
            let view = if index == 0 {
                View::Files
            } else {
                View::Settings
            };
            if ui.files.view != view {
                tx.settle();
                ui.open = None;
                ui.files.view = view;
                ui.files.armed = None;
                ui.files.editing = None;
            }
        },
    ))
}

pub(super) fn pane(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> (Vec<El>, El) {
    if ui.files.view == View::Settings {
        let content = super::controls::page("content").children(settings::view(ui, p, layer));
        return (
            Vec::new(),
            super::scroll_body("files-settings-scroll", content),
        );
    }
    let pane = &ui.files;
    let mut notices = Vec::new();
    if let Some(error) = &pane.error {
        notices.push(dismissible_error_panel(
            p,
            "files-error",
            tr("The file couldn't be added."),
            error,
            |ui: &mut Ui| ui.files.error = None,
        ));
    }
    let mut list = col().key("files-list");
    let visible = pane.visible();
    let mut day: Option<&str> = None;
    for (index, listed) in visible.iter().enumerate() {
        if day != Some(listed.day.as_str()) {
            list = list.child(ld::group(
                p,
                &format!("day{index}"),
                day_label(&listed.day),
                day.is_none(),
            ));
            day = Some(&listed.day);
        }
        list = list.child(list_row(pane, p, listed));
    }
    if visible.is_empty() && !pane.records.is_empty() {
        list = list.child(
            text(tr("No files match."))
                .key("no-match")
                .font(DESCRIPTION)
                .color(p.faint())
                .px(super::controls::METRICS.row_px),
        );
    }
    let detail = match pane.selected() {
        Some(record) => detail::view(ui, p, record, layer),
        None if pane.records.is_empty() => ld::empty_state(
            p,
            "files-empty",
            ld::HISTORY_ICON,
            tr("No files yet."),
            Some(tr(
                "Drop an audio or video file here to transcribe it, clean it up, and rework the text.",
            )),
        ),
        None => ld::empty_state(
            p,
            "files-detail",
            ld::HISTORY_ICON,
            tr("Select a file."),
            None,
        ),
    };
    let top = col()
        .key("files-top")
        .gap(10.0)
        .child(drop_zone(ui, p))
        .child(search(ui, p));
    let column = ld::list_column(top, "files-scroll", list);
    let body = col()
        .key("files-pane")
        .grow(1.0)
        .shrink()
        .children((!notices.is_empty()).then(|| ld::notices(notices)))
        .child(ld::body(p, "files-body", column, detail))
        // A drop anywhere on the pane adds the files.
        .on_drop_files(|ui: &mut Ui, files: &[PathBuf], _: &mut Tx| {
            ui.files.drop_over = false;
            add(ui, files);
        })
        .accepts_extensions(decode::EXTENSIONS)
        .on_drop_hover(|ui: &mut Ui, over, _: &mut Tx| ui.files.drop_over = over)
        .on_drop_rejected(|ui: &mut Ui, files: &[PathBuf], _: &mut Tx| {
            ui.files.drop_rejected = false;
            if let Some(file) = files.first() {
                ui.files.error = Some(tr_fill(
                    "{} isn't an audio or video file Hex can read.",
                    &file
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ));
            }
        })
        .on_drop_rejected_hover(|ui: &mut Ui, over, _: &mut Tx| ui.files.drop_rejected = over);
    (Vec::new(), body)
}

fn day_label(day: &str) -> &str {
    match day {
        "Today" => tr("Today"),
        "Yesterday" => tr("Yesterday"),
        other => other,
    }
}

/// The dashed drop target with Choose file…, lit while a drag is over the
/// pane.
fn drop_zone(ui: &Ui, p: Paint) -> El {
    let c = &p.k.c;
    let (title, ink, border) = if ui.files.drop_rejected {
        (tr("Hex can't read this file"), p.danger(), c.danger.border)
    } else if ui.files.drop_over {
        (tr("Drop to transcribe"), p.accent(), c.info.border)
    } else {
        (tr("Drop audio or video"), p.text(), c.border_base)
    };
    row()
        .key("files-drop")
        .gap(10.0)
        .center_y()
        .px(12.0)
        .py(10.0)
        .radius(p.k.radius.md)
        .border(1.0, border)
        .border_dash(4.0, 3.0)
        .fill(if ui.files.drop_over {
            c.info.bg
        } else {
            rune::Color::TRANSPARENT
        })
        .child(rune::icon(&UPLOAD_ICON, 18.0).key("icon").color(p.muted()))
        .child(
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .gap(2.0)
                .child(
                    text(title)
                        .key("title")
                        .font(p.k.ty.ui_medium)
                        .color(ink)
                        .truncate(),
                )
                .child(
                    text("mp3, m4a, wav, mov, mp4…")
                        .key("formats")
                        .font(p.k.ty.small)
                        .color(p.faint())
                        .truncate(),
                ),
        )
        .child(
            rune_kit::Button::new(tr("Choose file…"))
                .variant(rune_kit::ButtonVariant::Neutral)
                .control_size(rune_kit::ControlSize::Small)
                .on_press_tx(choose)
                .build(p.k)
                .key("files-choose"),
        )
}

static UPLOAD_ICON: rune::Icon = rune::Icon {
    name: "hex-files-upload",
    view: 16.0,
    body: r#"<path d="M8 10.5V2.5M5 5.3 8 2.3l3 3M2.5 10v3h11v-3" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

fn search(ui: &Ui, p: Paint) -> El {
    TextInput::new(&ui.files.search)
        .placeholder(tr("Search files"))
        .width(None)
        .leading_icon(p.k.glyph.search)
        .on_edit(|ui: &mut Ui| &mut ui.files.search)
        .build(p.k)
        .key("files-search")
}

/// A queued job's place in the queue, the running one first: `2nd in line`.
fn in_line(place: usize) -> String {
    match place {
        1 => tr("1st in line").into(),
        2 => tr("2nd in line").into(),
        3 => tr("3rd in line").into(),
        place => tr_fill("{}th in line", &place.to_string()),
    }
}

/// What a job is doing, for a list row and the progress card.
pub(super) fn stage_label(task: &Task) -> String {
    match task.stage {
        Stage::Queued => tr("Waiting").into(),
        Stage::Extracting => tr("Extracting audio").into(),
        Stage::Transcribing => tr("Transcribing").into(),
        Stage::CleaningUp => tr("Cleaning up").into(),
        Stage::Working => task.name.clone().unwrap_or_else(|| tr("Working").into()),
    }
}

fn list_row(pane: &FilesPane, p: Paint, listed: &Listed) -> El {
    let record = &listed.record;
    let task = pane.tasks_of(&record.id).next();
    let length = (record.duration_ms > 0).then(|| file_text::length(record.duration_ms));
    let mut bar = None;
    let (meta, ink, time) = if let Some(error) = record
        .error
        .clone()
        .or_else(|| worker::error(&record.id).filter(|_| !pane.fixture))
        && task.is_none()
    {
        (error, Some(p.danger()), listed.clock.clone())
    } else if let Some(task) = task {
        let percent = (task.fraction * 100.0).round() as u32;
        let with_length = |what: String| {
            length
                .clone()
                .map_or(what.clone(), |length| format!("{length}, {what}"))
        };
        let (meta, time) = match task.stage {
            Stage::Queued => {
                let place = pane
                    .tasks
                    .iter()
                    .position(|other| other.id == task.id)
                    .unwrap_or(0)
                    + 1;
                (with_length(in_line(place)), tr("Queued").to_string())
            }
            Stage::Extracting => (stage_label(task), format!("{percent}%")),
            Stage::Transcribing => (
                length.clone().map_or_else(
                    || stage_label(task),
                    |length| format!("{}, {length}", stage_label(task)),
                ),
                format!("{percent}%"),
            ),
            Stage::Working | Stage::CleaningUp => (
                with_length(format!("{} {percent}%", stage_label(task))),
                listed.clock.clone(),
            ),
        };
        if task.stage != Stage::Queued {
            bar = Some(task.fraction);
        }
        (meta, None, time)
    } else if record.versions.is_empty() {
        (tr("Stopped").into(), None, listed.clock.clone())
    } else {
        let count = record.versions.len();
        let versions = if count == 1 {
            tr("1 version").to_string()
        } else {
            tr_fill("{} versions", &count.to_string())
        };
        let meta = length.map_or(versions.clone(), |length| format!("{length}, {versions}"));
        (meta, None, listed.clock.clone())
    };
    let id = record.id.clone();
    ld::item(
        p,
        Item {
            key: format!("file-{}", record.id),
            title: record.name.clone(),
            meta,
            meta_ink: ink,
            time,
            hover: Vec::new(),
            selected: pane.selected.as_deref() == Some(record.id.as_str()),
            title_lines: 2,
            bar,
        },
        move |ui: &mut Ui| ui.files.select(id.clone()),
    )
}

/// A notice for `record`'s failure, with Try again when its file is there.
pub(super) fn failure(p: Paint, record: &FileRecord, error: &str) -> El {
    let id = record.id.clone();
    col()
        .key("file-failure")
        .gap(8.0)
        .child(error_panel(
            p,
            "file-error",
            tr("This file couldn't be transcribed."),
            error,
        ))
        .child(button(
            p,
            "file-retry",
            tr("Try again"),
            ButtonKind::Secondary,
            move |ui: &mut Ui, _| {
                let Some(record) = ui.files.record(&id) else {
                    return;
                };
                let id = record.id.clone();
                if !ui.files.fixture {
                    let _ = store::update(&id, |record| record.error = None);
                    worker::dismiss_error(&id);
                }
                let config = Config::from_settings(&ui.drafted());
                let job = Job::Transcribe {
                    selection: ui.settings.files.selection(&ui.settings.transcription),
                    model_name: config.transcription_name.clone(),
                    again: false,
                };
                submit(ui, &id, job);
            },
        ))
}

/// Opens `path` in the app that plays it.
pub(super) fn open_file(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    if let Err(error) = std::process::Command::new("/usr/bin/open")
        .arg(path)
        .spawn()
    {
        tracing::error!(%error, "couldn't open the file");
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
}

/// The app that opens `path` by default, such as `QuickTime Player`.
pub(super) fn opener(path: &std::path::Path) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSString, NSURL};
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
        let app = NSWorkspace::sharedWorkspace().URLForApplicationToOpenURL(&url)?;
        let name = app.lastPathComponent()?.to_string();
        Some(name.trim_end_matches(".app").to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

/// Reveals `path` in Finder.
pub(super) fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    if let Err(error) = std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(path)
        .spawn()
    {
        tracing::error!(%error, "couldn't reveal the file");
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
}
