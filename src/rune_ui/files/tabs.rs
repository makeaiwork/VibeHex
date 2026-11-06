//! A file's three tabs: Original, Cleanup, and Requests. Each holds its
//! versions (and the jobs that will add one); a tab with more than one shows
//! how many, and the open one lists them, newest first under their days.

use rune::{El, Shadow, TextStyle, col, el, icon, row, text};
use rune_kit::{Placement, Popover};

use super::super::controls::{DESCRIPTION, Paint};
use super::super::{Menu, Ui};
use super::{Tab, stage_label};
use crate::desktop::i18n::tr;
use crate::files::store::{FileRecord, Version, VersionKind};
use crate::files::worker::{Stage, Task};

static FROM_ICON: rune::Icon = rune::Icon {
    name: "hex-files-from",
    view: 16.0,
    body: r#"<path d="M4 2.5v6.5h8M9.5 6.5 12 9l-2.5 2.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

const VERSIONS: &str = "files-versions";

/// Where a version or job sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Group {
    Original,
    Cleanup,
    Requests,
}

impl Group {
    const ALL: [Group; 3] = [Group::Original, Group::Cleanup, Group::Requests];

    pub(super) fn of(kind: VersionKind) -> Group {
        match kind {
            VersionKind::Original => Group::Original,
            VersionKind::Cleanup => Group::Cleanup,
            VersionKind::Request => Group::Requests,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Group::Original => tr("Original"),
            Group::Cleanup => tr("Cleanup"),
            Group::Requests => tr("Requests"),
        }
    }
}

/// One of a group's versions, or a job that will add one.
#[derive(Clone, Copy)]
pub(super) enum Entry<'a> {
    Version(&'a Version),
    Task(&'a Task),
}

impl Entry<'_> {
    fn tab(&self) -> Tab {
        match self {
            Entry::Version(version) => Tab::Version(version.id),
            Entry::Task(task) => Tab::Task(task.id),
        }
    }
}

/// `group`'s versions, oldest first, then its jobs in queue order.
pub(super) fn entries<'a>(ui: &'a Ui, record: &'a FileRecord, group: Group) -> Vec<Entry<'a>> {
    record
        .versions
        .iter()
        .filter(|version| Group::of(version.kind) == group)
        .map(Entry::Version)
        .chain(
            ui.files
                .tasks_of(&record.id)
                .filter(move |task| Group::of(task.kind) == group && task.name.is_some())
                .map(Entry::Task),
        )
        .collect()
}

/// The group `tab` is in.
pub(super) fn group_of(ui: &Ui, record: &FileRecord, tab: Tab) -> Option<Group> {
    match tab {
        Tab::Version(id) => record.version(id).map(|version| Group::of(version.kind)),
        Tab::Task(id) => ui
            .files
            .tasks_of(&record.id)
            .find(|task| task.id == id)
            .map(|task| Group::of(task.kind)),
    }
}

/// A version's number in its group, from 1 by when it was made; `None`
/// while it is the group's only one.
pub(super) fn number(ui: &Ui, record: &FileRecord, version: &Version) -> Option<usize> {
    let group = entries(ui, record, Group::of(version.kind));
    if group.len() < 2 {
        return None;
    }
    group
        .iter()
        .position(|entry| entry.tab() == Tab::Version(version.id))
        .map(|at| at + 1)
}

/// A typed request: what was asked, as no quick request asks it.
pub(super) fn typed_prompt<'a>(ui: &Ui, version: &'a Version) -> Option<&'a str> {
    let prompt = version.prompt.as_deref()?.trim();
    let quick = crate::app_settings::prompt_defaults::is_quick_default(prompt)
        || ui
            .settings
            .files
            .quick_prompts
            .iter()
            .any(|chip| !chip.prompt.trim().is_empty() && chip.prompt.trim() == prompt);
    (!quick).then_some(prompt)
}

/// How a version is named in lists: its group for an Original or Cleanup,
/// a quick request's name, or a typed request's words in quotes.
pub(super) fn title(ui: &Ui, version: &Version) -> String {
    match version.kind {
        VersionKind::Original | VersionKind::Cleanup => Group::of(version.kind).label().into(),
        VersionKind::Request => match typed_prompt(ui, version) {
            Some(prompt) => format!("“{prompt}”"),
            None => version.name.clone(),
        },
    }
}

/// The name a version's tag shows on its line: a request's session name,
/// else "v2" when its group has more than one.
pub(super) fn tag(ui: &Ui, record: &FileRecord, version: &Version) -> Option<String> {
    if version.kind == VersionKind::Request {
        return Some(version.name.clone());
    }
    number(ui, record, version).map(|n| format!("v{n}"))
}

/// What a version was made from, as its tab's name.
pub(super) fn source(record: &FileRecord, version: &Version) -> Option<String> {
    let from = record.version(version.from?)?;
    Some(match from.kind {
        VersionKind::Request => from.name.clone(),
        kind => Group::of(kind).label().into(),
    })
}

/// Opens `group` on the version last opened there, else its newest.
fn open_group(ui: &mut Ui, record: &str, group: Group, tabs: &[Tab]) {
    let last = ui
        .files
        .last
        .get(&(record.to_string(), group))
        .copied()
        .filter(|tab| tabs.contains(tab));
    if let Some(tab) = last.or(tabs.last().copied()) {
        ui.files.tab = Some(tab);
    }
    ui.files.editing = None;
    ui.files.armed = None;
    ui.open = None;
}

/// Opens `tab`, remembered as its group's last until Hex quits.
fn open_version(ui: &mut Ui, record: &str, group: Group, tab: Tab) {
    ui.files.tab = Some(tab);
    ui.files.last.insert((record.to_string(), group), tab);
    ui.files.editing = None;
    ui.files.armed = None;
    ui.open = None;
}

/// The three tabs, when the file has more than one version or job.
pub(super) fn strip(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    tab: Option<Tab>,
    layer: &mut Vec<El>,
) -> Option<El> {
    let groups: Vec<(Group, Vec<Entry>)> = Group::ALL
        .into_iter()
        .map(|group| (group, entries(ui, record, group)))
        .filter(|(_, entries)| !entries.is_empty())
        .collect();
    if groups
        .iter()
        .map(|(_, entries)| entries.len())
        .sum::<usize>()
        < 2
    {
        return None;
    }
    let open = tab.and_then(|tab| group_of(ui, record, tab));
    let c = &p.k.c;
    let mut strip = row()
        .key("file-tabs")
        .role("tablist")
        .h(30.0)
        .center_y()
        .radius(7.0)
        .px(2.0)
        .gap(2.0)
        .fill(c.layer_01)
        .box_shadow(Shadow {
            spread: 0.5,
            color: c.border_base,
            ..Default::default()
        });
    for (group, entries) in groups {
        let on = open == Some(group);
        let tabs: Vec<Tab> = entries.iter().map(Entry::tab).collect();
        let working = entries.iter().any(|entry| matches!(entry, Entry::Task(_)));
        let label = group.label().to_string();
        let font = TextStyle {
            line_height: 13.0,
            ..p.k.ty.ui
        };
        let mut name = row()
            .key("name")
            .h(24.0)
            .pl(12.0)
            .pr(if on && entries.len() > 1 { 7.0 } else { 12.0 })
            .gap(6.0)
            .center_y()
            .children(working.then(|| rune_kit::Loader::new().size(11.0).build(p.k).key("working")))
            .child(text(label.clone()).key("label").font(font).color(if on {
                c.text_base
            } else {
                c.text_muted
            }));
        if entries.len() > 1 {
            name = name.child(
                row()
                    .key("count")
                    .h(15.0)
                    .px(5.0)
                    .center()
                    .radius(8.0)
                    .fill(if on { c.border_base } else { c.layer_03 })
                    .child(
                        text(entries.len().to_string())
                            .font(TextStyle {
                                line_height: 11.0,
                                ..p.k.ty.small_medium
                            })
                            .color(if on { c.text_base } else { c.text_muted }),
                    ),
            );
        }
        let id = record.id.clone();
        let current = tab;
        let name = name
            .role("tab")
            .label(label)
            .hover(c.hover)
            .radius(5.0)
            .on_press(move |ui: &mut Ui| {
                if let Some(current) = current
                    && let Some(from) = open
                {
                    ui.files.last.insert((id.clone(), from), current);
                }
                if !on {
                    open_group(ui, &id, group, &tabs);
                }
            });
        let mut cell = row().key(format!("tab-{group:?}")).radius(5.0).child(name);
        if on {
            cell = cell.fill(c.layer_02).box_shadow(Shadow {
                spread: 0.5,
                color: c.border_strong,
                ..Default::default()
            });
            if entries.len() > 1 {
                cell = cell
                    .center_y()
                    .child(el().key("divider").w(p.hair).h(14.0).fill(c.border_weak))
                    .child(versions_menu(ui, p, record, group, &entries, tab, layer));
            }
        }
        strip = strip.child(cell);
    }
    Some(row().key("file-tabs-row").child(strip))
}

/// The open tab's chevron and its list of versions under their days.
fn versions_menu(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    group: Group,
    entries: &[Entry],
    tab: Option<Tab>,
    layer: &mut Vec<El>,
) -> El {
    let c = &p.k.c;
    let open = ui.open == Some(Menu::Pick(VERSIONS));
    let chevron = row()
        .key("chevron")
        .role("button")
        .label(tr("Versions"))
        .h(24.0)
        .w(22.0)
        .center()
        .radius(5.0)
        .hover(c.hover)
        .child(
            icon(
                if open {
                    p.k.glyph.chevron_up
                } else {
                    p.k.glyph.chevron_down
                },
                12.0,
            )
            .color(c.text_base),
        )
        .on_press(|ui: &mut Ui| {
            ui.open = (ui.open != Some(Menu::Pick(VERSIONS))).then_some(Menu::Pick(VERSIONS));
        });
    let mut popover = Popover::new("files-versions-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .padding(4.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .trigger(chevron);
    let now = ui.files.now();
    let mut day = None;
    for (index, entry) in entries.iter().enumerate().rev() {
        let made = match entry {
            Entry::Version(version) => Some(version.created_ms),
            Entry::Task(_) => None,
        };
        let label = made.map(|made| crate::meeting::day_label(made, now));
        if label != day {
            if let Some(label) = &label {
                popover = popover.child(
                    text(super::day_label(label))
                        .key(format!("day{index}"))
                        .font(p.k.ty.small_medium)
                        .color(p.faint())
                        .px(10.0)
                        .pt(if day.is_none() { 4.0 } else { 8.0 })
                        .pb(2.0),
                );
            }
            day = label.clone();
        }
        popover = popover.child(entry_row(ui, p, record, group, *entry, tab, index));
    }
    popover.build(p.k, layer)
}

/// One version (or job) in the list: its name over its model and source,
/// its time at the end, ✓ on the open one.
fn entry_row(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    group: Group,
    entry: Entry,
    tab: Option<Tab>,
    index: usize,
) -> El {
    let c = &p.k.c;
    let on = tab == Some(entry.tab());
    let (name, model, from, time) = match entry {
        Entry::Version(version) => (
            title(ui, version),
            version.model.clone(),
            source(record, version),
            crate::meeting::clock(version.created_ms),
        ),
        Entry::Task(task) => (
            format!(
                "{}…",
                task.name.clone().unwrap_or_else(|| group.label().into())
            ),
            stage_label(task),
            None,
            if task.stage == Stage::Queued {
                String::new()
            } else {
                format!("{}%", (task.fraction * 100.0).round() as u32)
            },
        ),
    };
    let mut meta = row().key("meta").gap(8.0).center_y().child(
        text(model)
            .key("model")
            .font(p.k.ty.small)
            .color(p.muted())
            .shrink()
            .truncate(),
    );
    if let Some(from) = from {
        meta = meta.child(
            row()
                .key("from")
                .gap(3.0)
                .center_y()
                .child(icon(&FROM_ICON, 11.0).color(p.faint()))
                .child(
                    text(from)
                        .key("source")
                        .font(p.k.ty.small)
                        .color(p.muted())
                        .truncate(),
                ),
        );
    }
    let id = record.id.clone();
    let target = entry.tab();
    row()
        .key(format!("version{index}"))
        .role("menuitem")
        .label(name.clone())
        .w(264.0)
        .px(10.0)
        .py(6.0)
        .gap(8.0)
        .items_start()
        .radius(4.0)
        .fill(if on {
            c.hover
        } else {
            rune::Color::TRANSPARENT
        })
        .hover(c.hover)
        .child(
            el().key("check").size(14.0, 16.0).center().child(
                icon(p.k.glyph.check_small, 14.0)
                    .color(c.icon_base)
                    .opacity(if on { 1.0 } else { 0.0 }),
            ),
        )
        .child(
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .gap(2.0)
                .child(
                    text(name)
                        .key("name")
                        .font(p.k.ty.ui)
                        .color(p.text())
                        .wrap_anywhere()
                        .max_lines(2),
                )
                .child(meta),
        )
        .child(
            text(time)
                .key("time")
                .font(DESCRIPTION)
                .color(p.faint())
                .pt(1.0),
        )
        .on_press(move |ui: &mut Ui| open_version(ui, &id, group, target))
}
