//! A meeting's tabs, as Files has them: Transcript, Summary (the notes and
//! the chapters), and Requests (questions about the meeting), in a
//! strip under the meeting's line. A meeting that is still recording, or
//! has no notes yet, opens on its transcript.

mod chips;
pub(super) mod requests;

use rune::{El, Shadow, TextStyle, col, el, row, text};

use super::Ui;
use super::controls::{DESCRIPTION, Paint};
use super::meetings::{Meeting, Status};
use crate::desktop::i18n::tr;

/// The tabs a meeting can show.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MeetingTab {
    Summary,
    Transcript,
    Requests,
}

impl MeetingTab {
    fn label(self) -> &'static str {
        match self {
            Self::Summary => tr("Summary"),
            Self::Transcript => tr("Transcript"),
            Self::Requests => tr("Requests"),
        }
    }
}

/// The tabs `meeting` has: Summary once there are notes, chapters, or
/// notes on the way; Requests once it is finished, with the AI cleanup
/// engines on.
fn tabs(ui: &Ui, meeting: &Meeting) -> Vec<MeetingTab> {
    let notes = meeting.summary.is_some() || meeting.notes_running || !meeting.chapters.is_empty();
    // The transcript first, as Files puts the Original first.
    let mut tabs = vec![MeetingTab::Transcript];
    if notes {
        tabs.push(MeetingTab::Summary);
    }
    if requests::available(ui, meeting) {
        tabs.push(MeetingTab::Requests);
    }
    tabs
}

/// The open tab: the one chosen while it exists, else Summary for a
/// finished meeting with notes, else Transcript.
pub(super) fn open(ui: &Ui, meeting: &Meeting) -> MeetingTab {
    let tabs = tabs(ui, meeting);
    if let Some(chosen) = ui.meetings.tab.filter(|tab| tabs.contains(tab)) {
        return chosen;
    }
    let recording = matches!(meeting.status, Status::Starting | Status::Recording);
    if !recording && tabs.contains(&MeetingTab::Summary) {
        MeetingTab::Summary
    } else {
        MeetingTab::Transcript
    }
}

/// The strip, or none while the meeting has one tab.
pub(super) fn strip(ui: &Ui, p: Paint, meeting: &Meeting, layer: &mut Vec<El>) -> Option<El> {
    let tabs = tabs(ui, meeting);
    if tabs.len() < 2 {
        return None;
    }
    let open = open(ui, meeting);
    let c = &p.k.c;
    let mut strip = row()
        .key("meeting-tabs")
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
    for tab in tabs {
        let on = tab == open;
        let font = TextStyle {
            line_height: 13.0,
            ..p.k.ty.ui
        };
        let working = match tab {
            MeetingTab::Summary => meeting.notes_running,
            MeetingTab::Requests => requests::working(ui, meeting),
            MeetingTab::Transcript => false,
        };
        let count = match tab {
            MeetingTab::Requests => requests::count(ui, meeting),
            _ => 0,
        };
        let menu = on && count > 0;
        let mut name = row()
            .key("name")
            .h(24.0)
            .pl(12.0)
            .pr(if menu { 7.0 } else { 12.0 })
            .gap(6.0)
            .center_y()
            .children(working.then(|| rune_kit::Loader::new().size(11.0).build(p.k).key("working")))
            .child(text(tab.label()).key("label").font(font).color(if on {
                c.text_base
            } else {
                c.text_muted
            }));
        if count > 0 {
            name = name.child(
                row()
                    .key("count")
                    .h(15.0)
                    .px(5.0)
                    .center()
                    .radius(8.0)
                    .fill(if on { c.border_base } else { c.layer_03 })
                    .child(
                        text(count.to_string())
                            .font(TextStyle {
                                line_height: 11.0,
                                ..p.k.ty.small_medium
                            })
                            .color(if on { c.text_base } else { c.text_muted }),
                    ),
            );
        }
        let name = name
            .role("tab")
            .label(tab.label())
            .hover(c.hover)
            .radius(5.0)
            .on_press(move |ui: &mut Ui| ui.meetings.tab = Some(tab));
        let mut cell = row().key(format!("tab-{tab:?}")).radius(5.0).child(name);
        if on {
            cell = cell.fill(c.layer_02).box_shadow(Shadow {
                spread: 0.5,
                color: c.border_strong,
                ..Default::default()
            });
            if menu {
                cell = cell
                    .center_y()
                    .child(el().key("divider").w(p.hair).h(14.0).fill(c.border_weak))
                    .child(requests::sessions_menu(ui, p, layer));
            }
        }
        strip = strip.child(cell);
    }
    Some(row().key("meeting-tabs-row").child(strip))
}

/// The Summary tab: the notes, who wrote them, and the chapters.
pub(super) fn summary(p: Paint, meeting: &Meeting, chapters: Option<El>) -> El {
    let mut body = col().key("summary-tab").gap(24.0);
    if let Some(summary) = &meeting.summary {
        let summary = summary.trim();
        body = body.child(
            col()
                .key("notes")
                .gap(6.0)
                .child(if summary.is_empty() {
                    text(tr("Nothing to summarize."))
                        .key("summary")
                        .font(p.k.ty.body)
                        .color(p.faint())
                        .wrap()
                } else {
                    rune_kit::Markdown::new(summary.to_owned())
                        .style(
                            rune_kit::MarkdownStyle::from_kit(p.k)
                                .body_font(p.k.ty.body)
                                .text(p.reading()),
                        )
                        .selectable(true)
                        .tasks_read_only()
                        .build(p.k)
                        .key("summary")
                })
                .children(meeting.notes_by.as_deref().map(|by| {
                    text(by.to_owned())
                        .key("notes-by")
                        .font(p.k.ty.small)
                        .color(p.faint())
                        .wrap()
                        .mt(4.0)
                })),
        );
    } else if meeting.notes_running {
        body = body.child(
            text(tr("Writing a title and summary…"))
                .key("notes-running")
                .font(DESCRIPTION)
                .color(p.faint()),
        );
    }
    body.children(chapters)
}
