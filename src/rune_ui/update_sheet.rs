//! The update sheet: once an update is staged, the window shows which
//! version is ready, what it and every release skipped since the running one
//! changed, and Restart to update. It opens by itself once per staged
//! version, when nothing else holds the window (another sheet, the first
//! run, a shortcut recording); Not now closes it and the sidebar keeps
//! Restart to update. On macOS the menu-bar process also brings the window
//! up for it ([`crate::macos_rune`]).

use rune::{El, ScrollSpec, col, el, row, text};

use super::Ui;
use super::controls::{
    ButtonKind, DESCRIPTION, Paint, button, modal_scrim_transition, modal_transition,
};
use crate::desktop::host::{DesktopAction, DesktopUpdateStatus, ReleaseNotes, UpdateNotes};
use crate::desktop::i18n::{tr, tr_named};

const MODAL_WIDTH: f32 = 520.0;
const PADDING: f32 = 18.0;
/// The changes scroll past this height.
const NOTES_HEIGHT: f32 = 300.0;
/// The bottom fade of the scrolled changes.
const FADE: f32 = 8.0;

/// Whether the sheet shows, and the staged version it last opened for.
#[derive(Default)]
pub(super) struct UpdateSheet {
    pub(super) open: bool,
    announced: Option<String>,
}

/// The staged update the window knows of, while Restart to update shows.
fn staged(ui: &Ui) -> Option<&UpdateNotes> {
    ui.view
        .as_ref()
        .filter(|view| view.update_status == DesktopUpdateStatus::ReadyToRestart)
        .and_then(|view| view.update_notes.as_ref())
        .filter(|notes| !notes.version.is_empty())
}

/// Something else has the window: the sheet waits for it.
fn occupied(ui: &Ui) -> bool {
    ui.picker.open
        || ui.explainer.open
        || ui.sound_sets.open
        || ui.vocabulary.open
        || ui.first_run.is_some()
        || ui.tour.is_some()
        || !matches!(ui.capture, super::Capture::Idle)
}

/// Follows the host's update after each snapshot: opens the sheet once for
/// each newly staged version and closes it once none is staged. True when
/// it changed.
pub(super) fn follow(ui: &mut Ui) -> bool {
    let Some(version) = staged(ui).map(|notes| notes.version.clone()) else {
        return std::mem::take(&mut ui.update_sheet.open);
    };
    if ui.update_sheet.announced.as_deref() == Some(version.as_str()) || occupied(ui) {
        return false;
    }
    ui.update_sheet.announced = Some(version);
    ui.update_sheet.open = true;
    true
}

pub(super) fn close(ui: &mut Ui) {
    ui.update_sheet.open = false;
}

fn restart(ui: &mut Ui) {
    close(ui);
    ui.send(super::Request::Desktop(DesktopAction::RestartIntoUpdate));
}

/// One release: its version and date, then its changes.
fn release(p: Paint, index: usize, notes: &ReleaseNotes) -> El {
    let mut heading = row().key("heading").gap(8.0).center_y().child(
        text(notes.version.clone())
            .key("version")
            .font(p.k.ty.small_medium)
            .color(p.text()),
    );
    if let Some(date) = &notes.date {
        heading = heading.child(
            text(date.clone())
                .key("date")
                .font(p.k.ty.small)
                .color(p.faint()),
        );
    }
    let items = notes.items.iter().enumerate().map(|(item, change)| {
        text(format!("· {change}"))
            .key(format!("item{item}"))
            .font(p.k.ty.small)
            .color(p.muted())
            .wrap()
    });
    col()
        .key(format!("release{index}"))
        .gap(4.0)
        .child(heading)
        .children(items)
}

/// Every listed release, newest first, scrolling past [`NOTES_HEIGHT`].
fn releases(p: Paint, notes: &UpdateNotes) -> Option<El> {
    if notes.releases.is_empty() {
        return None;
    }
    let list = col().key("releases").gap(14.0).children(
        notes
            .releases
            .iter()
            .enumerate()
            .map(|(index, notes)| release(p, index, notes)),
    );
    Some(
        col()
            .key("changes")
            .shrink()
            .min_h(0.0)
            .max_h(NOTES_HEIGHT)
            .clip_fade(0.0, 0.0, 0.0, FADE)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(col().key("inset").pb(FADE).child(list)),
    )
}

pub(super) fn modal(ui: &Ui, p: Paint) -> Option<[El; 2]> {
    if !ui.update_sheet.open {
        return None;
    }
    let notes = staged(ui)?;
    let c = &p.k.c;
    let title = tr_named("VibeHex {version} is ready", &[("version", &notes.version)]);
    let heading = col()
        .key("heading")
        .gap(4.0)
        .child(
            text(title.clone())
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        )
        .child(
            text(tr_named(
                "You have {running}. Restart VibeHex to install it now, or it installs when you quit",
                &[("running", crate::VERSION)],
            ))
            .key("description")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap(),
        );
    let footer = row()
        .key("footer")
        .gap(12.0)
        .center_y()
        .child(el().key("spacer").grow(1.0))
        .child(button(
            p,
            "update-later",
            tr("Not now"),
            ButtonKind::Secondary,
            |ui: &mut Ui, _| close(ui),
        ))
        .child(
            button(
                p,
                "update-restart",
                tr("Restart to update"),
                ButtonKind::Primary,
                |ui: &mut Ui, _| restart(ui),
            )
            .default_button(),
        );
    let mut panel = col()
        .key("update-sheet")
        .role("dialog")
        .label(title)
        .w(MODAL_WIDTH)
        .pt(PADDING)
        .px(PADDING)
        .pb(14.0)
        .gap(14.0)
        .radius(p.k.radius.xl)
        .fill(c.bg_base)
        .border(p.hair, c.border_weak)
        // Presses inside the panel stay inside.
        .on_press(|_: &mut Ui| {})
        .child(heading);
    if let Some(changes) = releases(p, notes) {
        panel = panel.child(changes);
    }
    panel = panel.child(footer);
    for shadow in &p.k.elevation.overlay {
        panel = panel.box_shadow(*shadow);
    }
    let layer = |key: &str| {
        el().key(key)
            .role("none")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
    };
    Some([
        layer("update-sheet-scrim")
            .fill(c.scrim)
            .on_press(close)
            .transition(modal_scrim_transition(p)),
        layer("update-sheet-layer")
            .center()
            .transition(modal_transition(p))
            // Keys stay inside, and Escape or a press outside is Not now.
            .child(panel.modal(true, close)),
    ])
}

/// A staged update two releases ahead, with the sheet open.
pub(super) fn fixture(ui: &mut Ui) {
    let releases = vec![
        ReleaseNotes {
            version: "3.3.0-vibe.5".into(),
            date: Some("2026-11-05".into()),
            items: vec![
                "An update shows what changed, and every release skipped since yours".into(),
                "The built-in engine answers without reasoning".into(),
            ],
        },
        ReleaseNotes {
            version: "3.3.0-vibe.4".into(),
            date: Some("2026-11-04".into()),
            items: vec![
                "AI cleanup runs a downloaded model inside VibeHex (Built in)".into(),
                "Russian vibe coding turns on the two Labs it builds on".into(),
                "A Labs switch that writes the vocabulary reaches the window's editors".into(),
            ],
        },
    ];
    if let Some(view) = &mut ui.view {
        view.update_status = DesktopUpdateStatus::ReadyToRestart;
        view.update_notes = Some(UpdateNotes {
            items: releases
                .iter()
                .flat_map(|release| release.items.clone())
                .collect(),
            url: "https://github.com/makeaiwork/hex/releases".into(),
            version: "3.3.0-vibe.5".into(),
            releases,
        });
    }
    follow(ui);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staged_update(ui: &mut Ui, version: &str) {
        if let Some(view) = &mut ui.view {
            view.update_status = DesktopUpdateStatus::ReadyToRestart;
            view.update_notes = Some(UpdateNotes {
                version: version.into(),
                ..UpdateNotes::default()
            });
        }
    }

    fn mac() -> Ui {
        super::super::mac_fixture()
    }

    #[test]
    fn opens_once_for_each_newly_staged_version() {
        let mut ui = mac();
        assert!(!follow(&mut ui), "nothing staged");
        staged_update(&mut ui, "3.3.0-vibe.4");
        assert!(follow(&mut ui));
        assert!(ui.update_sheet.open);
        close(&mut ui);
        assert!(!follow(&mut ui), "Not now holds for that version");
        assert!(!ui.update_sheet.open);
        staged_update(&mut ui, "3.3.0-vibe.5");
        assert!(follow(&mut ui), "a newer staged release opens it again");
        if let Some(view) = &mut ui.view {
            view.update_status = DesktopUpdateStatus::Current;
        }
        assert!(follow(&mut ui));
        assert!(!ui.update_sheet.open, "closes once nothing is staged");
    }

    #[test]
    fn waits_while_something_else_holds_the_window() {
        let mut ui = mac();
        staged_update(&mut ui, "3.3.0-vibe.4");
        ui.vocabulary.open = true;
        assert!(!follow(&mut ui));
        ui.vocabulary.open = false;
        assert!(follow(&mut ui));
    }

    #[test]
    fn restart_to_update_closes_it_and_asks_the_host() {
        let mut ui = mac();
        staged_update(&mut ui, "3.3.0-vibe.4");
        follow(&mut ui);
        restart(&mut ui);
        assert!(!ui.update_sheet.open);
        let super::super::Service::Fixture(sent) = &ui.service else {
            panic!("a fixture records requests");
        };
        assert!(matches!(
            sent.as_slice(),
            [super::super::Request::Desktop(
                DesktopAction::RestartIntoUpdate
            )]
        ));
    }

    #[test]
    fn the_fixture_lists_both_releases() {
        let mut ui = mac();
        fixture(&mut ui);
        assert!(ui.update_sheet.open);
        let notes = staged(&ui).unwrap();
        assert_eq!(notes.releases.len(), 2);
    }
}
