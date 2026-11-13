//! About's version and updates: the version (a local or test build marks
//! itself beside it), then Check for updates with how the person's check
//! went, or an update to take, or why this copy doesn't update itself, and
//! under them this build's release notes (Labs › Release notes link).
//!
//! The sidebar shows only an update to take (`sidebar_update`); the menu
//! bar's Check for Updates opens this pane to say how it went (#1120).

use rune::{Color, El, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize};

use super::super::controls::{DESCRIPTION, Paint, status_dot};
use super::super::host::Request;
use super::super::{Ui, sidebar_update};
use crate::desktop::host::{DesktopAction, DesktopUpdateStatus};
use crate::desktop::i18n::{tr, tr_fill};

/// What kind of build this is, for the badge beside the version. The scenes
/// preview the others on a release build.
#[derive(Clone, Copy, PartialEq)]
pub(in super::super) enum BuildKind {
    Release,
    Local,
    Test,
}

fn build_kind() -> BuildKind {
    if crate::app_paths::is_local_build() {
        BuildKind::Local
    } else if option_env!("HEX_TEST_BUILD") == Some("1") {
        BuildKind::Test
    } else {
        BuildKind::Release
    }
}

/// This build's kind, or the one a scene previews.
fn kind(ui: &Ui) -> BuildKind {
    ui.build_preview.unwrap_or_else(build_kind)
}

/// The version; a local or test build's with its commit: main moves past a
/// release without changing the version.
fn version_words() -> String {
    // Goldens render a fixed version: a local build's names its commit.
    let version = if cfg!(test) { "0.0.0" } else { crate::VERSION };
    match option_env!("HEX_COMMIT") {
        Some(commit) => tr_fill("Version {}", &format!("{version} ({commit})")),
        None => tr_fill("Version {}", version),
    }
}

/// The version, and beside it a local or test build's mark: an outline pill
/// with a dot, in muted theme tokens.
pub(super) fn version(ui: &Ui, p: Paint) -> El {
    let version = text(version_words())
        .key("version")
        .font(DESCRIPTION)
        .color(p.muted());
    let badge = match kind(ui) {
        BuildKind::Release => return version,
        BuildKind::Local => tr("Local"),
        BuildKind::Test => tr("Test"),
    };
    let c = &p.k.c;
    row()
        .key("version-row")
        .gap(6.0)
        .center_y()
        .child(version.shrink().min_w(0.0))
        .child(
            row()
                .key("build-badge")
                .h(16.0)
                .px(6.0)
                .gap(4.0)
                .center_y()
                .radius(8.0)
                .border(p.hair.max(0.5), c.border_muted)
                .child(status_dot(c.text_faint).size(5.0, 5.0).radius(2.5))
                .child(
                    text(badge)
                        .key("label")
                        .font(p.k.ty.small)
                        .color(c.text_muted),
                ),
        )
}

/// A small button of the card, as About's others are.
fn small(label: &str) -> Button {
    Button::new(label)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
}

/// The words beside or instead of the button.
fn words(key: &str, words: &str, color: Color) -> El {
    text(words)
        .key(key.to_owned())
        .font(DESCRIPTION)
        .color(color)
        .wrap()
        .shrink()
}

/// Under the version: Check for updates and how the person's check went,
/// an update to take with its dot, or what keeps this copy from checking.
/// The hourly check alone never changes the words; the button keeps its
/// place in every state where a check can run, so a check moves nothing.
pub(super) fn control(ui: &Ui, p: Paint) -> Option<El> {
    use DesktopUpdateStatus as S;
    let status = &ui.view.as_ref()?.update_status;
    let line = || row().key("update").gap(10.0).mt(9.0).center_y();
    // An update to take: the sidebar's button, here too.
    if let Some((label, action)) = sidebar_update::action(ui) {
        let take = small(&label)
            .on_press(move |ui: &mut Ui| ui.send(Request::Desktop(action.clone())))
            .build(p.k)
            .key("update-take");
        return Some(
            line()
                .gap(8.0)
                .child(status_dot(p.accent()).key("dot"))
                .child(take),
        );
    }
    // Words alone read as the version's next line, without a button's air.
    let quiet = |said: &str| {
        Some(
            line()
                .mt(4.0)
                .child(words("update-status", said, p.faint())),
        )
    };
    match status {
        S::Current | S::Failed | S::Checking => {
            let checking = *status == S::Checking;
            let check = small(tr("Check for updates"))
                .disabled(checking)
                .on_press(|ui: &mut Ui| {
                    ui.update_asked = true;
                    ui.send(Request::Desktop(DesktopAction::CheckForUpdates));
                })
                .build(p.k)
                .key("update-check");
            // After a check the person asked for, how it went (#1120).
            let said = match status {
                _ if !ui.update_asked => None,
                S::Current => Some((tr("VibeHex is up to date"), p.muted())),
                S::Failed => Some((tr("Couldn't check for updates"), p.muted())),
                _ => Some((tr("Checking for updates…"), p.faint())),
            };
            Some(
                line()
                    .child(check)
                    .children(said.map(|(said, color)| words("update-status", said, color))),
            )
        }
        S::Managed => quiet(tr("Managed by your organization")),
        S::Downloading => quiet(tr("Downloading update")),
        S::SourceBuild if kind(ui) == BuildKind::Local => quiet(tr("Local build, updates off")),
        S::SourceBuild => quiet(tr("Updated from your source checkout")),
        S::PackageManager => quiet(tr("Updated by your package manager")),
        // The command that updates it, as the package manager has it.
        S::PackageUpdate { command } => quiet(command),
        S::Unavailable | S::Available | S::ReadyToRestart | S::SwitchAvailable { .. } => None,
    }
}

/// This build's release notes, under the update check while Labs › Release
/// notes link is on. Unlike What's new, it needs no staged update.
pub(super) fn release_notes(ui: &Ui, p: Paint) -> Option<El> {
    if !crate::labs::Feature::ReleaseNotesLink.on(&ui.settings) {
        return None;
    }
    // A source or test build has no published notes for its code.
    if kind(ui) != BuildKind::Release {
        return None;
    }
    // Tests pin a VibeHex release, whose notes the fork publishes.
    let version = if cfg!(test) {
        "0.0.0-vibe.1"
    } else {
        crate::VERSION
    };
    let url = crate::updater::notes::release_notes_url(version)?;
    let c = &p.k.c;
    Some(
        row()
            .key("release-notes")
            .h(20.0)
            .px(6.0)
            // The words stand on the version's edge; the hover reaches past.
            .ml(-6.0)
            .mt(4.0)
            .center_y()
            .self_start()
            .radius(p.k.radius.sm)
            .role("link")
            .label(tr("Release notes"))
            .hover(c.hover)
            .press(c.pressed)
            .focusable(true)
            .focus_ring(2.0, -2.0, c.border_focus)
            .on_press(move |ui: &mut Ui| super::open(ui, &url))
            .child(
                text(tr("Release notes"))
                    .key("label")
                    .font(DESCRIPTION)
                    .color(c.text_muted)
                    .truncate(),
            ),
    )
}
